//! Flash driver.
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use hazard3_hal_internal::{Peri, PeripheralType};
use embedded_storage::nor_flash::{
    ErrorType, MultiwriteNorFlash, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash, check_erase, check_read,
    check_write,
};

use crate::mode::{Async, Blocking, Mode};
use crate::peripherals::FLASH;
use crate::{dma, interrupt, mode, pac};

/// Flash base address.
pub const FLASH_BASE: *const u32 = 0x10000000 as _;

/// Address for xip setup function set up by the 235x bootrom.
#[cfg(feature = "_rp235x")]
pub const BOOTRAM_BASE: *const u32 = 0x400e0000 as _;

/// Copy the bootrom XIP entry to RAM unless the application already runs from RAM.
// TODO: when run-from-ram is set, completely skip the "pause cores and jumpp to RAM" dance.
pub const USE_BOOTRAM_FLASH_ENTRY: bool = !cfg!(feature = "run-from-ram");

/// Flash page size.
pub const PAGE_SIZE: usize = 256;
/// Flash write size.
pub const WRITE_SIZE: usize = 1;
/// Flash read size.
pub const READ_SIZE: usize = 1;
/// Flash erase size.
pub const ERASE_SIZE: usize = 4096;
/// Flash DMA read size.
pub const ASYNC_READ_SIZE: usize = 4;

/// Error type for NVMC operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Operation using a location not in flash.
    OutOfBounds,
    /// Unaligned operation or using unaligned buffers.
    Unaligned,
    /// Accessed from the wrong core.
    InvalidCore,
    /// Other error
    Other,
}

impl From<NorFlashErrorKind> for Error {
    fn from(e: NorFlashErrorKind) -> Self {
        match e {
            NorFlashErrorKind::NotAligned => Self::Unaligned,
            NorFlashErrorKind::OutOfBounds => Self::OutOfBounds,
            _ => Self::Other,
        }
    }
}

impl NorFlashError for Error {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Self::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            Self::Unaligned => NorFlashErrorKind::NotAligned,
            _ => NorFlashErrorKind::Other,
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfBounds => f.write_str("Out of bounds"),
            Self::Unaligned => f.write_str("Unaligned"),
            Self::InvalidCore => f.write_str("Invalid core"),
            Self::Other => f.write_str("Other error"),
        }
    }
}

impl core::error::Error for Error {}

/// Future that waits for completion of a background read
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct BackgroundRead<'a, 'd, const FLASH_SIZE: usize> {
    flash: PhantomData<&'a mut Flash<'d, Async, FLASH_SIZE>>,
    transfer: dma::Transfer<'a>,
}

impl<'a, 'd, const FLASH_SIZE: usize> Future for BackgroundRead<'a, 'd, FLASH_SIZE> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.transfer).poll(cx)
    }
}

impl<'a, 'd, const FLASH_SIZE: usize> Drop for BackgroundRead<'a, 'd, FLASH_SIZE> {
    fn drop(&mut self) {
        if pac::XIP_CTRL.stream_ctr().read().0 == 0 {
            return;
        }
        pac::XIP_CTRL
            .stream_ctr()
            .write_value(pac::xip_ctrl::regs::StreamCtr(0));
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        // An uncached read stalls until the stream transfer is complete.
        const XIP_NOCACHE_NOALLOC_BASE: *const u32 = 0x14000000 as *const _;
        unsafe {
            core::ptr::read_volatile(XIP_NOCACHE_NOALLOC_BASE);
        }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

/// Flash driver.
pub struct Flash<'d, M: Mode, const FLASH_SIZE: usize> {
    dma: M::IfAsync<dma::Channel<'d, mode::Async>>,
    phantom: PhantomData<(&'d mut (), M)>,
}

impl<'d, M: Mode, const FLASH_SIZE: usize> Flash<'d, M, FLASH_SIZE> {
    /// Blocking read.
    ///
    /// The offset and buffer must be aligned.
    ///
    /// NOTE: `offset` is an offset from the flash start, NOT an absolute address.
    pub fn blocking_read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Error> {
        trace!(
            "Reading from 0x{:x} to 0x{:x}",
            FLASH_BASE as u32 + offset,
            FLASH_BASE as u32 + offset + bytes.len() as u32
        );
        check_read(self, offset, bytes.len())?;

        let flash_data = unsafe { core::slice::from_raw_parts((FLASH_BASE as u32 + offset) as *const u8, bytes.len()) };

        bytes.copy_from_slice(flash_data);
        Ok(())
    }

    /// Flash capacity.
    pub fn capacity(&self) -> usize {
        FLASH_SIZE
    }

    /// Blocking erase.
    ///
    /// NOTE: `offset` is an offset from the flash start, NOT an absolute address.
    pub fn blocking_erase(&mut self, from: u32, to: u32) -> Result<(), Error> {
        check_erase(self, from, to)?;

        trace!(
            "Erasing from 0x{:x} to 0x{:x}",
            FLASH_BASE as u32 + from,
            FLASH_BASE as u32 + to
        );

        let len = to - from;

        unsafe { in_ram(|| ram_helpers::flash_range_erase(from, len))? };

        Ok(())
    }

    /// Blocking write.
    ///
    /// The offset and buffer must be aligned.
    ///
    /// NOTE: `offset` is an offset from the flash start, NOT an absolute address.
    pub fn blocking_write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Error> {
        check_write(self, offset, bytes.len())?;

        trace!("Writing {:?} bytes to 0x{:x}", bytes.len(), FLASH_BASE as u32 + offset);

        let end_offset = offset as usize + bytes.len();

        let padded_offset = (offset as *const u8).align_offset(PAGE_SIZE);
        let start_padding = core::cmp::min(padded_offset, bytes.len());

        // Pad in the beginning
        if start_padding > 0 {
            let start = PAGE_SIZE - padded_offset;
            let end = start + start_padding;

            let mut pad_buf = [0xFF_u8; PAGE_SIZE];
            pad_buf[start..end].copy_from_slice(&bytes[..start_padding]);

            let unaligned_offset = offset as usize - start;

            unsafe { in_ram(|| ram_helpers::flash_range_program(unaligned_offset as u32, &pad_buf))? }
        }

        let remaining_len = bytes.len() - start_padding;
        let end_padding = start_padding + PAGE_SIZE * (remaining_len / PAGE_SIZE);

        // Write aligned slice of length in multiples of 256 bytes
        // If the remaining bytes to be written is more than a full page.
        if remaining_len >= PAGE_SIZE {
            let mut aligned_offset = if start_padding > 0 {
                offset as usize + padded_offset
            } else {
                offset as usize
            };

            if bytes.as_ptr() as usize >= 0x2000_0000 {
                let aligned_data = &bytes[start_padding..end_padding];

                unsafe { in_ram(|| ram_helpers::flash_range_program(aligned_offset as u32, aligned_data))? }
            } else {
                for chunk in bytes[start_padding..end_padding].chunks_exact(PAGE_SIZE) {
                    let mut ram_buf = [0xFF_u8; PAGE_SIZE];
                    ram_buf.copy_from_slice(chunk);
                    unsafe { in_ram(|| ram_helpers::flash_range_program(aligned_offset as u32, &ram_buf))? }
                    aligned_offset += PAGE_SIZE;
                }
            }
        }

        // Pad in the end
        let rem_offset = (end_offset as *const u8).align_offset(PAGE_SIZE);
        let rem_padding = remaining_len % PAGE_SIZE;
        if rem_padding > 0 {
            let mut pad_buf = [0xFF_u8; PAGE_SIZE];
            pad_buf[..rem_padding].copy_from_slice(&bytes[end_padding..]);

            let unaligned_offset = end_offset - (PAGE_SIZE - rem_offset);

            unsafe { in_ram(|| ram_helpers::flash_range_program(unaligned_offset as u32, &pad_buf))? }
        }

        Ok(())
    }
}

impl<'d, const FLASH_SIZE: usize> Flash<'d, Blocking, FLASH_SIZE> {
    /// Create a new flash driver in blocking mode.
    pub fn new_blocking<T: Instance>(_flash: Peri<'d, T>) -> Self {
        Self {
            dma: (),
            phantom: PhantomData,
        }
    }
}

impl<'d, const FLASH_SIZE: usize> Flash<'d, Async, FLASH_SIZE> {
    /// Create a new flash driver in async mode.
    pub fn new<T: Instance, D: dma::ChannelInstance>(
        _flash: Peri<'d, T>,
        dma: Peri<'d, D>,
        irq: impl interrupt::typelevel::Binding<D::Interrupt, dma::InterruptHandler<D>> + 'd,
    ) -> Self {
        Self {
            dma: dma::Channel::new(dma, irq),
            phantom: PhantomData,
        }
    }

    /// Start a background read operation.
    ///
    /// The offset and buffer must be aligned.
    ///
    /// NOTE: `offset` is an offset from the flash start, NOT an absolute address.
    pub fn background_read<'a>(
        &'a mut self,
        offset: u32,
        data: &'a mut [u32],
    ) -> Result<BackgroundRead<'a, 'd, FLASH_SIZE>, Error> {
        trace!(
            "Reading in background from 0x{:x} to 0x{:x}",
            FLASH_BASE as u32 + offset,
            FLASH_BASE as u32 + offset + (data.len() * 4) as u32
        );
        // Can't use check_read because we need to enforce 4-byte alignment
        let offset = offset as usize;
        let length = data.len() * 4;
        if length > self.capacity() || offset > self.capacity() - length {
            return Err(Error::OutOfBounds);
        }
        if !offset.is_multiple_of(4) {
            return Err(Error::Unaligned);
        }

        while !pac::XIP_CTRL.stat().read().fifo_empty() {
            pac::XIP_CTRL.stream_fifo().read();
        }

        pac::XIP_CTRL
            .stream_addr()
            .write_value(pac::xip_ctrl::regs::StreamAddr(FLASH_BASE as u32 + offset as u32));
        pac::XIP_CTRL
            .stream_ctr()
            .write_value(pac::xip_ctrl::regs::StreamCtr(data.len() as u32));

        // Use the XIP AUX bus port, rather than the FIFO register access (e.x.
        // pac::XIP_CTRL.stream_fifo().as_ptr()) to avoid DMA stalling on
        // general XIP access.
        const XIP_AUX_BASE: *const u32 = 0x50500000 as *const _;
        let transfer = unsafe {
            self.dma
                .read(XIP_AUX_BASE, data, pac::dma::vals::TreqSel::XipStream, false)
        };

        Ok(BackgroundRead {
            flash: PhantomData,
            transfer,
        })
    }

    /// Async read.
    ///
    /// The offset and buffer must be aligned.
    ///
    /// NOTE: `offset` is an offset from the flash start, NOT an absolute address.
    pub async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Error> {
        use core::mem::MaybeUninit;

        // Checked early to simplify address validity checks
        if !bytes.len().is_multiple_of(4) {
            return Err(Error::Unaligned);
        }

        // If the destination address is already aligned, then we can just DMA directly
        if (bytes.as_ptr() as u32).is_multiple_of(4) {
            // Safety: alignment and size have been checked for compatibility
            let buf: &mut [u32] =
                unsafe { core::slice::from_raw_parts_mut(bytes.as_mut_ptr() as *mut u32, bytes.len() / 4) };
            self.background_read(offset, buf)?.await;
            return Ok(());
        }

        // Destination address is unaligned, so use an intermediate buffer
        const REALIGN_CHUNK: usize = PAGE_SIZE;
        // Safety: MaybeUninit requires no initialization
        let mut buf: [MaybeUninit<u32>; REALIGN_CHUNK / 4] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut chunk_offset: usize = 0;
        while chunk_offset < bytes.len() {
            let chunk_size = (bytes.len() - chunk_offset).min(REALIGN_CHUNK);
            let buf = &mut buf[..(chunk_size / 4)];

            // Safety: this is written to completely by DMA before any reads
            let buf = unsafe { &mut *(buf as *mut [MaybeUninit<u32>] as *mut [u32]) };
            self.background_read(offset + chunk_offset as u32, buf)?.await;

            // Safety: [u8] has more relaxed alignment and size requirements than [u32], so this is just aliasing
            let buf = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const _, buf.len() * 4) };
            bytes[chunk_offset..(chunk_offset + chunk_size)].copy_from_slice(&buf[..chunk_size]);

            chunk_offset += chunk_size;
        }

        Ok(())
    }
}

impl<'d, M: Mode, const FLASH_SIZE: usize> ErrorType for Flash<'d, M, FLASH_SIZE> {
    type Error = Error;
}

impl<'d, M: Mode, const FLASH_SIZE: usize> ReadNorFlash for Flash<'d, M, FLASH_SIZE> {
    const READ_SIZE: usize = READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.blocking_read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<'d, M: Mode, const FLASH_SIZE: usize> MultiwriteNorFlash for Flash<'d, M, FLASH_SIZE> {}

impl<'d, const FLASH_SIZE: usize> embedded_storage_async::nor_flash::MultiwriteNorFlash
    for Flash<'d, Async, FLASH_SIZE>
{
}

impl<'d, M: Mode, const FLASH_SIZE: usize> NorFlash for Flash<'d, M, FLASH_SIZE> {
    const WRITE_SIZE: usize = WRITE_SIZE;

    const ERASE_SIZE: usize = ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.blocking_erase(from, to)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.blocking_write(offset, bytes)
    }
}

impl<'d, const FLASH_SIZE: usize> embedded_storage_async::nor_flash::ReadNorFlash for Flash<'d, Async, FLASH_SIZE> {
    const READ_SIZE: usize = ASYNC_READ_SIZE;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.read(offset, bytes).await
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<'d, const FLASH_SIZE: usize> embedded_storage_async::nor_flash::NorFlash for Flash<'d, Async, FLASH_SIZE> {
    const WRITE_SIZE: usize = WRITE_SIZE;

    const ERASE_SIZE: usize = ERASE_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.blocking_erase(from, to)
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.blocking_write(offset, bytes)
    }
}

#[allow(dead_code)]
pub(crate) mod ram_helpers {
    use super::*;
    use crate::rom_data;

    #[repr(C)]
    struct FlashFunctionPointers<'a> {
        connect_internal_flash: unsafe extern "C" fn() -> (),
        flash_exit_xip: unsafe extern "C" fn() -> (),
        flash_range_erase: Option<unsafe extern "C" fn(addr: u32, count: usize, block_size: u32, block_cmd: u8) -> ()>,
        flash_range_program: Option<unsafe extern "C" fn(addr: u32, data: *const u8, count: usize) -> ()>,
        flash_flush_cache: unsafe extern "C" fn() -> (),
        flash_enter_cmd_xip: unsafe extern "C" fn() -> (),
        phantom: PhantomData<&'a ()>,
    }

    #[allow(unused)]
    fn flash_function_pointers(erase: bool, write: bool) -> FlashFunctionPointers<'static> {
        FlashFunctionPointers {
            connect_internal_flash: rom_data::connect_internal_flash::ptr(),
            flash_exit_xip: rom_data::flash_exit_xip::ptr(),
            flash_range_erase: if erase {
                Some(rom_data::flash_range_erase::ptr())
            } else {
                None
            },
            flash_range_program: if write {
                Some(rom_data::flash_range_program::ptr())
            } else {
                None
            },
            flash_flush_cache: rom_data::flash_flush_cache::ptr(),
            flash_enter_cmd_xip: rom_data::flash_enter_cmd_xip::ptr(),
            phantom: PhantomData,
        }
    }

    #[allow(unused)]
    /// # Safety
    ///
    /// `bootram_entry` must contain the RP235x bootrom entry that re-initializes XIP mode.
    unsafe fn flash_function_pointers_with_bootram_entry(
        erase: bool,
        write: bool,
        bootram_entry: &[u32; 64],
    ) -> FlashFunctionPointers<'_> {
        let entry_ptr = (bootram_entry as *const u32 as *const u8).offset(1);
        let entry: unsafe extern "C" fn() -> () = core::mem::transmute(entry_ptr);
        FlashFunctionPointers {
            connect_internal_flash: rom_data::connect_internal_flash::ptr(),
            flash_exit_xip: rom_data::flash_exit_xip::ptr(),
            flash_range_erase: if erase {
                Some(rom_data::flash_range_erase::ptr())
            } else {
                None
            },
            flash_range_program: if write {
                Some(rom_data::flash_range_program::ptr())
            } else {
                None
            },
            flash_flush_cache: rom_data::flash_flush_cache::ptr(),
            flash_enter_cmd_xip: entry,
            phantom: PhantomData,
        }
    }

    /// Erase a flash range starting at `addr` with length `len`.
    ///
    /// `addr` and `len` must be multiples of 4096
    ///
    /// If `USE_BOOTRAM_FLASH_ENTRY` is `true`, the RP235x bootrom flash entry is copied to RAM
    /// and used to re-initialize XIP after flashing.
    ///
    /// # Safety
    ///
    /// Nothing must access flash while this is running.
    /// Usually this means:
    ///   - interrupts must be disabled
    ///   - 2nd core must be running code from RAM or ROM with interrupts disabled
    ///   - DMA must not access flash memory
    ///
    /// `addr` and `len` parameters must be valid and are not checked.
    pub unsafe fn flash_range_erase(addr: u32, len: u32) {
        let mut bootram_entry = [0u32; 256 / 4];
        let ptrs = if USE_BOOTRAM_FLASH_ENTRY {
            core::ptr::copy_nonoverlapping(BOOTRAM_BASE as *const u8, bootram_entry.as_mut_ptr() as *mut u8, 256);
            flash_function_pointers_with_bootram_entry(true, false, &bootram_entry)
        } else {
            flash_function_pointers(true, false)
        };

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        write_flash_inner(addr, len, None, &ptrs as *const FlashFunctionPointers);
    }

    /// Erase and rewrite a flash range starting at `addr` with data `data`.
    ///
    /// `addr` and `data.len()` must be multiples of 4096
    ///
    /// If `USE_BOOTRAM_FLASH_ENTRY` is `true`, the RP235x bootrom flash entry is copied to RAM
    /// and used to re-initialize XIP after flashing.
    ///
    /// # Safety
    ///
    /// Nothing must access flash while this is running.
    /// Usually this means:
    ///   - interrupts must be disabled
    ///   - 2nd core must be running code from RAM or ROM with interrupts disabled
    ///   - DMA must not access flash memory
    ///
    /// `addr` and `len` parameters must be valid and are not checked.
    pub unsafe fn flash_range_erase_and_program(addr: u32, data: &[u8]) {
        let mut bootram_entry = [0u32; 256 / 4];
        let ptrs = if USE_BOOTRAM_FLASH_ENTRY {
            core::ptr::copy_nonoverlapping(BOOTRAM_BASE as *const u8, bootram_entry.as_mut_ptr() as *mut u8, 256);
            flash_function_pointers_with_bootram_entry(true, true, &bootram_entry)
        } else {
            flash_function_pointers(true, true)
        };

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        write_flash_inner(
            addr,
            data.len() as u32,
            Some(data),
            &ptrs as *const FlashFunctionPointers,
        );
    }

    /// Write a flash range starting at `addr` with data `data`.
    ///
    /// `addr` and `data.len()` must be multiples of 256
    ///
    /// If `USE_BOOTRAM_FLASH_ENTRY` is `true`, the RP235x bootrom flash entry is copied to RAM
    /// and used to re-initialize XIP after flashing.
    ///
    /// # Safety
    ///
    /// Nothing must access flash while this is running.
    /// Usually this means:
    ///   - interrupts must be disabled
    ///   - 2nd core must be running code from RAM or ROM with interrupts disabled
    ///   - DMA must not access flash memory
    ///
    /// `addr` and `len` parameters must be valid and are not checked.
    pub unsafe fn flash_range_program(addr: u32, data: &[u8]) {
        let mut bootram_entry = [0u32; 256 / 4];
        let ptrs = if USE_BOOTRAM_FLASH_ENTRY {
            core::ptr::copy_nonoverlapping(BOOTRAM_BASE as *const u8, bootram_entry.as_mut_ptr() as *mut u8, 256);
            flash_function_pointers_with_bootram_entry(false, true, &bootram_entry)
        } else {
            flash_function_pointers(false, true)
        };

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        write_flash_inner(
            addr,
            data.len() as u32,
            Some(data),
            &ptrs as *const FlashFunctionPointers,
        );
    }

    /// # Safety
    ///
    /// Nothing must access flash while this is running.
    /// Usually this means:
    ///   - interrupts must be disabled
    ///   - 2nd core must be running code from RAM or ROM with interrupts disabled
    ///   - DMA must not access flash memory
    ///
    /// Length of data must be a multiple of 4096
    /// addr must be aligned to 4096
    #[inline(never)]
    #[unsafe(link_section = ".data.ram_func")]
    #[cfg(feature = "_rp235x")]
    unsafe fn write_flash_inner(addr: u32, len: u32, data: Option<&[u8]>, ptrs: *const FlashFunctionPointers) {
        let data = data.map(|d| d.as_ptr()).unwrap_or(core::ptr::null());
        ((*ptrs).connect_internal_flash)();
        ((*ptrs).flash_exit_xip)();
        if (*ptrs).flash_range_erase.is_some() {
            ((*ptrs).flash_range_erase.unwrap())(addr, len as usize, 1 << 31, 0);
        }
        if (*ptrs).flash_range_program.is_some() {
            ((*ptrs).flash_range_program.unwrap())(addr, data as *const _, len as usize);
        }
        ((*ptrs).flash_flush_cache)();
        ((*ptrs).flash_enter_cmd_xip)();
    }
}

/// Make sure flash operations uphold their safety contract.
/// - interrupts must be disabled
/// - DMA must not access flash memory
pub(crate) unsafe fn in_ram(operation: impl FnOnce()) -> Result<(), Error> {
    // Make sure we're running on CORE0
    let core_id: u32 = pac::SIO.cpuid().read();
    if core_id != 0 {
        return Err(Error::InvalidCore);
    }

    // Make sure CORE1 is paused during the entire duration of the RAM function
    crate::multicore::pause_core1();

    critical_section::with(|_| {
        // Wait for all DMA channels in flash to finish before ram operation
        const SRAM_LOWER: u32 = 0x2000_0000;
        for n in 0..crate::dma::CHANNEL_COUNT {
            let ch = crate::pac::DMA.ch(n);
            while ch.read_addr().read() < SRAM_LOWER && ch.ctrl_trig().read().busy() {}
        }
        // Wait for completion of any background reads
        while pac::XIP_CTRL.stream_ctr().read().0 > 0 {}

        // On RP235x the bootrom flash routines reset QMI window 1
        // (CS1) to default XIP timings, which clobbers any custom
        // configuration applied for an attached PSRAM. Snapshot the
        // window-1 registers and restore them afterwards so PSRAM
        // access keeps working across flash erase/program.
        // See pico-sdk#1983.
        #[cfg(feature = "_rp235x")]
        let qmi_m1 = {
            let m = pac::QMI.mem(1);
            (
                m.timing().read(),
                m.rfmt().read(),
                m.rcmd().read(),
                m.wfmt().read(),
                m.wcmd().read(),
            )
        };

        // Clean the XIP cache before invoking the bootrom flash
        // routines. The bootrom calls `flash_flush_cache`, which
        // INVALIDATES every cache line by set/way — that silently
        // drops any dirty PSRAM lines (writes the CPU made that
        // hadn't yet been written back to physical PSRAM via the
        // cache write-back path). Cleaning first pushes those writes
        // out so the subsequent invalidate is harmless. See
        // pico-sdk's `hardware_xip_cache::xip_cache_clean_all` and
        // erratum RP2350-E11.
        //
        // Encoding: addr[2:0]=0x1 selects "clean by set/way";
        // addr[12:3] is the set index (1024 sets), addr[13] is the
        // way (2 ways) — 2048 lines total. Maintenance writes are
        // anchored at the very top of the maintenance window
        // (0x1BFFC000) so the bogus tag left in the line by erratum
        // E11 (`clean by set/way` writes the maintenance address
        // into the line tag) sits in a region nothing reads.
        #[cfg(feature = "_rp235x")]
        {
            const MAINT_BASE: usize = 0x1BFFC000;
            let mut i: usize = 0;
            while i < 2048 {
                let addr = MAINT_BASE + (i * 8) + 1;
                core::ptr::write_volatile(addr as *mut u8, 0);
                i += 1;
            }
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        }

        // Run our flash operation in RAM
        operation();

        #[cfg(feature = "_rp235x")]
        {
            let m = pac::QMI.mem(1);
            m.timing().write_value(qmi_m1.0);
            m.rfmt().write_value(qmi_m1.1);
            m.rcmd().write_value(qmi_m1.2);
            m.wfmt().write_value(qmi_m1.3);
            m.wcmd().write_value(qmi_m1.4);
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        }
    });

    // Resume CORE1 execution
    crate::multicore::resume_core1();
    Ok(())
}

trait SealedInstance {}

/// Flash instance.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType {}

impl SealedInstance for FLASH {}
impl Instance for FLASH {}
