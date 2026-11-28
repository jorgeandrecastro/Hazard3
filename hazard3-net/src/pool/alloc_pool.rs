use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::ptr::NonNull;

use super::{Align, Pool, Slot, ValidAlign, check_buf_capacity};
use crate::driver::{PacketBuf, PacketMeta, RawPacketBuf};

/// A [`Pool`] that gets its buffers from the heap and recycles them.
///
/// At most `max_count` buffers are allocated from the heap in total. A buffer
/// that is dropped is not given back to the heap: it is kept in a free list and
/// reused by the next `alloc`. In steady state, `alloc` and `drop` therefore never
/// touch the heap allocator and never clear the buffer storage. This makes them
/// fast and constant-time, and it avoids heap fragmentation.
///
/// The flip side is that the heap memory of a buffer is only released by
/// [`trim`](Self::trim). Until then, the pool holds up to `max_count` buffers.
///
/// If the heap runs out before `max_count` is reached, the allocation aborts,
/// like any other heap allocation. Make sure the heap has room for `max_count`
/// buffers.
///
/// The free list is protected by a [`critical_section`]. On RISC-V and Cortex-M
/// this is a couple of instructions, with no retry loop, so the latency is
/// bounded. Your firmware must provide a `critical-section` implementation (the
/// HAL of your chip usually does).
///
/// [`StaticPool`](super::StaticPool) explains how to pick `SIZE`.
///
/// `ALIGN` can be 1, 2, 4, 8, 16, 32 or 64.
///
/// The build fails for the same buffer sizes that fail with `StaticPool`.
///
/// It must be `'static`. It can go in a `static`:
///
/// ```
/// use hazard3_net::{AllocPool, Stack};
///
/// static POOL: AllocPool<1514> = AllocPool::new(64);
///
/// let stack = Stack::new(&POOL, 0x1234_5678);
/// ```
pub struct AllocPool<const SIZE: usize = 1514, const ALIGN: usize = 1>
where
    Align<ALIGN>: ValidAlign,
{
    max_count: usize,
    /// Only accessed inside a critical section, see [`Self::with`].
    state: UnsafeCell<State>,
}

/// The mutable part of the pool.
struct State {
    /// Head of the free list. Each cached slot starts with a link to the next one.
    head: Option<NonNull<()>>,
    /// Number of slots allocated from the heap: in use plus cached.
    total: usize,
}

/// What `take` found.
enum Take {
    /// The pool is at `max_count` and has no cached slot.
    Exhausted,
    /// The caller must allocate a new slot from the heap. It is already counted.
    Fresh,
    /// A cached slot, removed from the free list.
    Recycled(NonNull<()>),
}

// SAFETY: `state` is only accessed inside a critical section, and the slots it
// points to are only reachable through the free list or through one `PacketBuf`.
unsafe impl<const SIZE: usize, const ALIGN: usize> Sync for AllocPool<SIZE, ALIGN> where
    Align<ALIGN>: ValidAlign
{
}

impl<const SIZE: usize, const ALIGN: usize> AllocPool<SIZE, ALIGN>
where
    Align<ALIGN>: ValidAlign,
{
    /// Create a pool that allocates at most `max_count` buffers from the heap.
    pub const fn new(max_count: usize) -> Self {
        const { check_buf_capacity(SIZE) };
        Self {
            max_count,
            state: UnsafeCell::new(State {
                head: None,
                total: 0,
            }),
        }
    }

    /// Give the heap memory of all cached buffers back to the allocator.
    ///
    /// Buffers that are in use are not affected. After this, the pool can
    /// allocate new buffers from the heap again, up to `max_count` in total.
    ///
    /// Each buffer is released in its own short critical section, so this does
    /// not hold interrupts off for long.
    pub fn trim(&self) {
        loop {
            let slot = self.with(|s| {
                let slot = s.head?;
                // SAFETY: a cached slot starts with the link written by `put`.
                s.head = unsafe { slot.cast::<Option<NonNull<()>>>().read() };
                s.total -= 1;
                Some(slot)
            });
            match slot {
                // SAFETY: the slot came from `Box::into_raw` in `alloc`, and it
                // was in the free list, so nothing else uses it.
                Some(slot) => drop(unsafe {
                    Box::from_raw(slot.cast::<Slot<SIZE, ALIGN>>().as_ptr())
                }),
                None => break,
            }
        }
    }

    /// Run `f` with exclusive access to the state.
    #[inline(always)]
    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        critical_section::with(|_| {
            // SAFETY: the critical section gives exclusive access to the state.
            f(unsafe { &mut *self.state.get() })
        })
    }

    /// Pop a cached slot, or reserve room for a new one.
    #[inline]
    fn take(&self) -> Take {
        self.with(|s| match s.head {
            Some(slot) => {
                // SAFETY: a cached slot starts with the link written by `put`.
                s.head = unsafe { slot.cast::<Option<NonNull<()>>>().read() };
                Take::Recycled(slot)
            }
            None if s.total < self.max_count => {
                s.total += 1;
                Take::Fresh
            }
            None => Take::Exhausted,
        })
    }

    /// Push a slot on the free list.
    ///
    /// # Safety
    /// `slot` must be a slot of this pool that nothing uses anymore.
    #[inline]
    unsafe fn put(&self, slot: NonNull<()>) {
        self.with(|s| {
            // The link overwrites the start of the slot, where `RawPacketBuf` was.
            // `alloc` writes it again in full. The storage is left alone.
            // SAFETY: a slot is at least as big and as aligned as a pointer.
            unsafe { slot.cast::<Option<NonNull<()>>>().write(s.head) };
            s.head = Some(slot);
        });
    }

    /// Give a buffer back to its pool. Called when it is dropped.
    ///
    /// # Safety
    /// `raw` must be the `RawPacketBuf` `alloc` gave the buffer, and the buffer must
    /// no longer be used.
    unsafe fn free(raw: NonNull<RawPacketBuf>) {
        // SAFETY: `alloc` set `raw.pool` to a `&'static Self`. `raw` is the first
        // field of a slot of that pool. `pool` is read before `put` overwrites `raw`.
        unsafe {
            let pool = &*raw.as_ref().pool.cast::<Self>();
            pool.put(raw.cast());
        }
    }
}

impl<const SIZE: usize, const ALIGN: usize> Pool for AllocPool<SIZE, ALIGN>
where
    Align<ALIGN>: ValidAlign,
{
    #[inline(never)] // helps code size: it has many callers
    fn alloc(&'static self) -> Option<PacketBuf> {
        let slot = match self.take() {
            Take::Exhausted => return None,
            Take::Recycled(slot) => slot.cast::<Slot<SIZE, ALIGN>>().as_ptr(),
            // Zeroed, so the storage is initialized. Only new slots pay for this.
            Take::Fresh => Box::into_raw(Box::<Slot<SIZE, ALIGN>>::new_zeroed()).cast(),
        };
        // SAFETY: the slot is ours, and its storage is initialized: it is zeroed
        // or it holds data from earlier packets.
        unsafe {
            let data = (&raw mut (*slot).data).cast::<u8>();
            // Catch code that relies on fresh buffers being zeroed.
            #[cfg(test)]
            data.write_bytes(0xa5, SIZE);
            let raw = (&raw mut (*slot).raw).cast::<RawPacketBuf>();
            raw.write(RawPacketBuf {
                free: Self::free,
                pool: (self as *const Self).cast(),
                data: NonNull::new_unchecked(data),
                capacity: SIZE,
                headroom: 0,
                len: 0,
                meta: PacketMeta::default(),
            });
            Some(PacketBuf::from_raw(NonNull::new_unchecked(raw)))
        }
    }

    fn buf_capacity(&self) -> usize {
        SIZE
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::mem::{align_of, offset_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::vec::Vec;

    use super::*;
    use crate::pool::PoolRef;

    /// Declare a fresh `static` pool. Every test needs its own: tests run in
    /// parallel threads and a shared pool would make the counters meaningless.
    macro_rules! pool {
        ($size:expr, $align:expr, $max:expr) => {{
            static POOL: AllocPool<{ $size }, { $align }> = AllocPool::new($max);
            &POOL
        }};
    }

    // Inspection helpers, only for tests.
    impl<const SIZE: usize, const ALIGN: usize> AllocPool<SIZE, ALIGN>
    where
        Align<ALIGN>: ValidAlign,
    {
        /// Number of slots owned by the pool (in use + cached).
        fn total(&self) -> usize {
            self.with(|s| s.total)
        }

        /// Number of slots in the free list. Walks the list, and panics if it is
        /// longer than `total`, which means it is corrupted or has a cycle.
        fn cached(&self) -> usize {
            self.with(|s| {
                let mut n = 0;
                let mut cur = s.head;
                while let Some(slot) = cur {
                    n += 1;
                    assert!(n <= s.total, "free list is longer than the slot count");
                    // SAFETY: a cached slot starts with the link written by `put`.
                    cur = unsafe { slot.cast::<Option<NonNull<()>>>().read() };
                }
                n
            })
        }

        /// The invariant that must hold whenever no call is in progress.
        fn assert_invariants(&self, in_use: usize) {
            assert_eq!(self.total(), self.cached() + in_use, "total != cached + in use");
            assert!(self.total() <= self.max_count, "total exceeds max_count");
        }
    }

    #[test]
    fn new_pool_is_empty() {
        let pool = pool!(1514, 1, 4);
        assert_eq!(pool.total(), 0);
        assert_eq!(pool.cached(), 0);
        assert_eq!(pool.buf_capacity(), 1514);
    }

    #[test]
    fn zero_max_count_never_allocates() {
        let pool = pool!(1514, 1, 0);
        assert!(pool.alloc().is_none());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn allocates_up_to_max_count_then_none() {
        let pool = pool!(1514, 1, 3);
        let bufs: Vec<_> = (0..3).map(|_| pool.alloc().expect("buffer")).collect();
        pool.assert_invariants(3);
        assert!(pool.alloc().is_none(), "pool must be exhausted");
        assert!(pool.alloc().is_none(), "exhaustion must be stable");
        pool.assert_invariants(3);
        drop(bufs);
        pool.assert_invariants(0);
    }

    #[test]
    fn every_buffer_has_the_pool_capacity() {
        let pool = pool!(1514, 1, 4);
        let bufs: Vec<_> = (0..4).map(|_| pool.alloc().unwrap()).collect();
        for b in &bufs {
            assert_eq!(b.capacity() as usize, pool.buf_capacity());
        }
    }

    #[test]
    fn dropped_buffer_is_cached_not_released() {
        let pool = pool!(1514, 1, 4);
        let buf = pool.alloc().unwrap();
        assert_eq!((pool.total(), pool.cached()), (1, 0));
        drop(buf);
        // The slot is kept in the free list, the heap memory is not released.
        assert_eq!((pool.total(), pool.cached()), (1, 1));
    }

    #[test]
    fn recycling_does_not_grow_the_pool() {
        let pool = pool!(1514, 1, 4);
        for _ in 0..1000 {
            let buf = pool.alloc().unwrap();
            assert_eq!(buf.capacity() as usize, 1514, "recycled buffer lost its capacity");
            drop(buf);
        }
        // One slot was enough for all the cycles.
        pool.assert_invariants(0);
        assert_eq!(pool.total(), 1);
    }

    #[test]
    fn freeing_makes_room_when_exhausted() {
        let pool = pool!(1514, 1, 2);
        let a = pool.alloc().unwrap();
        let _b = pool.alloc().unwrap();
        assert!(pool.alloc().is_none());
        drop(a);
        let c = pool.alloc().expect("a freed slot must be reusable");
        assert!(pool.alloc().is_none());
        drop(c);
        pool.assert_invariants(1);
    }

    #[test]
    fn free_list_survives_any_drop_order() {
        let pool = pool!(1514, 1, 8);
        let mut bufs: Vec<_> = (0..8).map(|_| pool.alloc().unwrap()).collect();
        // Drop the even positions first, then the odd ones, to mix the list order.
        let mut odd = Vec::new();
        for (i, b) in bufs.drain(..).enumerate() {
            if i % 2 == 0 {
                drop(b);
            } else {
                odd.push(b);
            }
        }
        pool.assert_invariants(4);
        drop(odd);
        pool.assert_invariants(0);
        assert_eq!(pool.cached(), 8);
        // All 8 slots must be handed out again, and no more.
        let again: Vec<_> = (0..8).map(|_| pool.alloc().expect("recycled")).collect();
        assert!(pool.alloc().is_none());
        assert_eq!(pool.total(), 8, "recycling must not allocate new slots");
        drop(again);
    }

    #[test]
    fn forgotten_buffer_stays_counted() {
        let pool = pool!(1514, 1, 2);
        core::mem::forget(pool.alloc().unwrap());
        let _b = pool.alloc().unwrap();
        // A leaked buffer is never returned, so it still uses one of the 2 slots.
        assert!(pool.alloc().is_none());
        assert_eq!(pool.total(), 2);
    }

    #[test]
    fn trim_on_an_empty_pool_does_nothing() {
        let pool = pool!(1514, 1, 4);
        pool.trim();
        pool.assert_invariants(0);
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn trim_releases_only_cached_slots() {
        let pool = pool!(1514, 1, 4);
        let bufs: Vec<_> = (0..4).map(|_| pool.alloc().unwrap()).collect();
        let mut it = bufs.into_iter();
        let keep = [it.next().unwrap(), it.next().unwrap()];
        drop(it); // two slots go back to the free list
        assert_eq!((pool.total(), pool.cached()), (4, 2));

        pool.trim();
        assert_eq!((pool.total(), pool.cached()), (2, 0));

        // The two buffers still in use are untouched and still work.
        for b in &keep {
            assert_eq!(b.capacity() as usize, 1514);
        }
        drop(keep);
        pool.assert_invariants(0);
    }

    #[test]
    fn pool_is_usable_after_trim() {
        let pool = pool!(1514, 1, 2);
        drop((pool.alloc().unwrap(), pool.alloc().unwrap()));
        pool.trim();
        assert_eq!(pool.total(), 0);
        // Trimming must give the whole budget back.
        let a = pool.alloc().unwrap();
        let b = pool.alloc().unwrap();
        assert!(pool.alloc().is_none());
        pool.assert_invariants(2);
        drop((a, b));
    }

    #[test]
    fn works_through_a_type_erased_pool_ref() {
        let pool = pool!(1514, 1, 2);
        let r = PoolRef::new(pool);
        let a = r.alloc().unwrap();
        let b = r.clone().alloc().unwrap();
        assert!(r.alloc().is_none());
        assert_eq!(pool.total(), 2);
        drop((a, b));
        pool.assert_invariants(0);
    }

    #[test]
    fn works_with_the_largest_alignment() {
        let pool = pool!(1514, 64, 2);
        let a = pool.alloc().unwrap();
        let b = pool.alloc().unwrap();
        assert_eq!(a.capacity() as usize, 1514);
        drop((a, b));
        pool.assert_invariants(0);
    }

    /// Layout rules the free list and `free` depend on.
    fn check_layout<const A: usize>()
    where
        Align<A>: ValidAlign,
    {
        assert!(align_of::<Slot<1514, A>>() >= A);
        assert_eq!(offset_of!(Slot<1514, A>, data) % A, 0, "data not aligned to {A}");
        // `free` turns a `*RawPacketBuf` back into a slot pointer.
        assert_eq!(offset_of!(Slot<1514, A>, raw), 0, "`raw` must stay the first field");
    }

    #[test]
    fn slot_layout_is_valid_for_every_alignment() {
        check_layout::<1>();
        check_layout::<2>();
        check_layout::<4>();
        check_layout::<8>();
        check_layout::<16>();
        check_layout::<32>();
        check_layout::<64>();
    }

    #[test]
    fn slot_can_hold_the_free_list_link() {
        // `put` writes an `Option<NonNull<()>>` at the start of a free slot.
        assert!(size_of::<RawPacketBuf>() >= size_of::<Option<NonNull<()>>>());
        assert!(align_of::<Slot<1514, 1>>() >= align_of::<Option<NonNull<()>>>());
        assert_eq!(size_of::<Option<NonNull<()>>>(), size_of::<usize>());
    }

    #[test]
    fn pool_struct_stays_small() {
        // max_count + head + total. Guards against fields added by accident: a
        // pool lives in RAM, and there can be several.
        assert!(size_of::<AllocPool<1514, 1>>() <= 3 * size_of::<usize>());
    }

    #[test]
    fn concurrent_alloc_and_drop_respect_max_count() {
        const MAX: usize = 5;
        static POOL: AllocPool<1514, 1> = AllocPool::new(MAX);
        static LIVE: AtomicUsize = AtomicUsize::new(0);
        static OK: AtomicUsize = AtomicUsize::new(0);

        let threads: Vec<_> = (0..8)
            .map(|_| {
                thread::spawn(|| {
                    for i in 0..5_000 {
                        if let Some(buf) = POOL.alloc() {
                            let live = LIVE.fetch_add(1, Ordering::SeqCst) + 1;
                            assert!(live <= MAX, "more buffers in use than max_count");
                            OK.fetch_add(1, Ordering::Relaxed);
                            if i % 64 == 0 {
                                thread::yield_now();
                            }
                            // Decrement first: the slot can be reused right after the
                            // drop, so the counter must never run ahead of reality.
                            LIVE.fetch_sub(1, Ordering::SeqCst);
                            drop(buf);
                        }
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().expect("worker thread panicked");
        }

        assert!(OK.load(Ordering::Relaxed) > 0);
        assert_eq!(LIVE.load(Ordering::SeqCst), 0);
        POOL.assert_invariants(0);
        assert!(POOL.total() <= MAX);
    }
}