// SPDX-License-Identifier: MIT OR Apache-2.0

//! TCP client compatible with `embedded-nal-async` traits.
//!
//! Requires the `embedded-nal` feature (gated in the parent module).

use core::cell::{Cell, UnsafeCell};
use core::mem::MaybeUninit;
use core::net::IpAddr;
use core::ptr::NonNull;

use hazard3_time::Duration;
use hazard3_net::error::Full;

use super::{Error, TcpSocket};
use crate::Stack;

/// TCP client connection pool compatible with `embedded-nal-async` traits.
///
/// The pool is capable of managing up to N concurrent connections with tx and rx buffers according to TX_SZ and RX_SZ.
pub struct TcpClient<'d, const N: usize, const TX_SZ: usize = 1024, const RX_SZ: usize = 1024> {
    stack: Stack<'d>,
    state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
    socket_timeout: Option<Duration>,
}

impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> TcpClient<'d, N, TX_SZ, RX_SZ> {
    /// Create a new `TcpClient`.
    pub fn new(stack: Stack<'d>, state: &'d TcpClientState<N, TX_SZ, RX_SZ>) -> Self {
        Self {
            stack,
            state,
            socket_timeout: None,
        }
    }

    /// Set the timeout for each socket created by this `TcpClient`.
    ///
    /// If the timeout is set, the socket will be closed if no data is received for the
    /// specified duration.
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.socket_timeout = timeout;
    }
}

impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_nal_async::TcpConnect
    for TcpClient<'d, N, TX_SZ, RX_SZ>
{
    type Error = Error;
    type Connection<'m>
        = TcpConnection<'m, 'd, N, TX_SZ, RX_SZ>
    where
        Self: 'm;

    async fn connect<'a>(&'a self, remote: core::net::SocketAddr) -> Result<Self::Connection<'a>, Self::Error> {
        let addr: crate::wire::IpAddr = match remote.ip() {
            #[cfg(feature = "ipv4")]
            IpAddr::V4(addr) => crate::wire::IpAddr::V4(addr),
            #[cfg(not(feature = "ipv4"))]
            IpAddr::V4(_) => panic!("ipv4 support not enabled"),
            #[cfg(feature = "ipv6")]
            IpAddr::V6(addr) => crate::wire::IpAddr::V6(addr),
            #[cfg(not(feature = "ipv6"))]
            IpAddr::V6(_) => panic!("ipv6 support not enabled"),
        };
        let remote_addr = (addr, remote.port());
        let mut socket = TcpConnection::new(self.stack, self.state)?;
        socket.socket.set_timeout(self.socket_timeout);
        socket
            .socket
            .connect(remote_addr)
            .await
            .map_err(|_| Error::ConnectionReset)?;
        Ok(socket)
    }
}

/// Opened TCP connection in a [`TcpClient`].
///
/// `'a` is the lifetime of the borrow of the [`TcpClient`] (whose pool the
/// connection's buffers come from), `'d` the lifetime of the stack.
pub struct TcpConnection<'a, 'd, const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
    socket: TcpSocket<'a, 'd>,
    state: &'a TcpClientState<N, TX_SZ, RX_SZ>,
    bufs: NonNull<([u8; TX_SZ], [u8; RX_SZ])>,
}

impl<'a, 'd, const N: usize, const TX_SZ: usize, const RX_SZ: usize> TcpConnection<'a, 'd, N, TX_SZ, RX_SZ> {
    fn new(stack: Stack<'d>, state: &'a TcpClientState<N, TX_SZ, RX_SZ>) -> Result<Self, Error> {
        let mut bufs = state.pool.alloc().ok_or(Error::ConnectionReset)?;
        let socket = unsafe { TcpSocket::new(stack, &mut bufs.as_mut().1, &mut bufs.as_mut().0) };
        let socket = match socket {
            Ok(socket) => socket,
            Err(Full) => {
                unsafe { state.pool.free(bufs) };
                return Err(Error::ConnectionReset);
            }
        };
        Ok(Self { socket, state, bufs })
    }
}

impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> Drop for TcpConnection<'_, '_, N, TX_SZ, RX_SZ> {
    fn drop(&mut self) {
        unsafe {
            self.socket.close();
            self.state.pool.free(self.bufs);
        }
    }
}

impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::ErrorType
    for TcpConnection<'_, '_, N, TX_SZ, RX_SZ>
{
    type Error = Error;
}

impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Read
    for TcpConnection<'_, '_, N, TX_SZ, RX_SZ>
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.socket.read(buf).await
    }
}

impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Write
    for TcpConnection<'_, '_, N, TX_SZ, RX_SZ>
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.socket.write(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.socket.flush().await
    }
}

/// State for TcpClient
pub struct TcpClientState<const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
    pool: Pool<([u8; TX_SZ], [u8; RX_SZ]), N>,
}

impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> TcpClientState<N, TX_SZ, RX_SZ> {
    /// Create a new `TcpClientState`.
    pub const fn new() -> Self {
        Self { pool: Pool::new() }
    }
}

struct Pool<T, const N: usize> {
    used: [Cell<bool>; N],
    data: [UnsafeCell<MaybeUninit<T>>; N],
}

impl<T, const N: usize> Pool<T, N> {
    const VALUE: Cell<bool> = Cell::new(false);
    const UNINIT: UnsafeCell<MaybeUninit<T>> = UnsafeCell::new(MaybeUninit::uninit());

    const fn new() -> Self {
        Self {
            used: [Self::VALUE; N],
            data: [Self::UNINIT; N],
        }
    }
}

impl<T, const N: usize> Pool<T, N> {
    fn alloc(&self) -> Option<NonNull<T>> {
        for n in 0..N {
            // this can't race because Pool is not Sync.
            if !self.used[n].get() {
                self.used[n].set(true);
                let p = self.data[n].get() as *mut T;
                return Some(unsafe { NonNull::new_unchecked(p) });
            }
        }
        None
    }

    /// safety: p must be a pointer obtained from self.alloc that hasn't been freed yet.
    unsafe fn free(&self, p: NonNull<T>) {
        let origin = self.data.as_ptr() as *mut T;
        let n = p.as_ptr().offset_from(origin);
        assert!(n >= 0);
        assert!((n as usize) < N);
        self.used[n as usize].set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_exhausts_then_returns_none() {
        let pool: Pool<u32, 3> = Pool::new();
        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_none());
    }

    #[test]
    fn pool_zero_capacity() {
        let pool: Pool<u32, 0> = Pool::new();
        assert!(pool.alloc().is_none());
    }

    #[test]
    fn pool_slots_are_distinct_and_dont_alias() {
        let pool: Pool<u32, 4> = Pool::new();
        let mut ptrs = [None; 4];
        for (i, slot) in ptrs.iter_mut().enumerate() {
            let p = pool.alloc().unwrap();
            unsafe { p.as_ptr().write(i as u32 * 10) };
            *slot = Some(p);
        }
        for (i, slot) in ptrs.iter().enumerate() {
            let p = slot.unwrap();
            assert_eq!(unsafe { p.as_ptr().read() }, i as u32 * 10);
            for other in &ptrs[i + 1..] {
                assert_ne!(p, other.unwrap());
            }
        }
    }

    #[test]
    fn pool_free_makes_slot_reusable() {
        let pool: Pool<u32, 2> = Pool::new();
        let a = pool.alloc().unwrap();
        let b = pool.alloc().unwrap();
        assert!(pool.alloc().is_none());

        unsafe { pool.free(b) };
        let b2 = pool.alloc().unwrap();
        assert_eq!(b, b2);
        assert!(pool.alloc().is_none());

        unsafe {
            pool.free(a);
            pool.free(b2);
        }
    }

    #[test]
    fn pool_reuses_lowest_free_slot_first() {
        let pool: Pool<u32, 3> = Pool::new();
        let a = pool.alloc().unwrap();
        let _b = pool.alloc().unwrap();
        let c = pool.alloc().unwrap();

        // Free out of order: highest first, then lowest.
        unsafe {
            pool.free(c);
            pool.free(a);
        }
        assert_eq!(pool.alloc().unwrap(), a);
        assert_eq!(pool.alloc().unwrap(), c);
        assert!(pool.alloc().is_none());
    }

    #[test]
    fn client_state_pool_matches_buffer_layout() {
        const TX: usize = 8;
        const RX: usize = 4;
        let state = TcpClientState::<2, TX, RX>::new();

        let a = state.pool.alloc().unwrap();
        let b = state.pool.alloc().unwrap();
        assert!(state.pool.alloc().is_none());

        // Consecutive slots are exactly one `(tx, rx)` buffer pair apart.
        let spacing = (b.as_ptr() as usize).abs_diff(a.as_ptr() as usize);
        assert_eq!(spacing, size_of::<([u8; TX], [u8; RX])>());
    }

    #[test]
    fn client_state_is_const_constructible() {
        const _STATE: TcpClientState<1, 16, 16> = TcpClientState::new();
    }
}