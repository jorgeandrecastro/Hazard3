// SPDX-License-Identifier: MIT OR Apache-2.0

//! TCP sockets.
//!
//! Module layout:
//! - `tcp.rs` (this file): error types and the shared `TcpIo` helper.
//! - `tcp/socket.rs`: [`TcpSocket`] construction, connect/accept, options, state.
//! - `tcp/socket/io.rs`: data transfer, [`TcpReader`], [`TcpWriter`], `embedded-io` impls.
//! - `tcp/listener.rs`: [`TcpListener`] (feature `tcp-listener`).
//! - `tcp/client.rs`: `embedded-nal-async` client (feature `embedded-nal`).

use core::future::{Future, poll_fn};
use core::task::{Context, Poll};

#[cfg(feature = "iface-bind")]
pub use hazard3_net::iface::IfaceHandle;
#[cfg(feature = "tcp-listener")]
pub use hazard3_net::tcp::AcceptToken;
pub use hazard3_net::tcp::State;
use hazard3_net::tcp::{self, TcpHandle};

use crate::{NoWake, Stack, TryError, Wake, WakeRunner};

mod socket;
pub use socket::{TcpReader, TcpSocket, TcpWriter};

#[cfg(feature = "tcp-listener")]
mod listener;
#[cfg(feature = "tcp-listener")]
pub use listener::TcpListener;

#[cfg(feature = "embedded-nal")]
pub mod client;

/// Error returned by TcpSocket read/write functions.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The connection was reset.
    ///
    /// This can happen on receiving a RST packet, or on timeout.
    ConnectionReset,
}

/// Error returned by [`TcpSocket::connect`]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectError {
    /// The socket is already open.
    InvalidState,
    /// The remote port is zero, the remote address is unspecified, or there is
    /// no route to the remote host or no local address to reach it from.
    Unaddressable,
    /// No free port in the ephemeral range (only possible with tens of thousands
    /// of open sockets).
    NoFreePorts,
    /// Another TCP socket already holds the identical 4-tuple.
    InUse,
    /// The remote host rejected the connection with a RST packet.
    ConnectionReset,
    /// Connect timed out.
    TimedOut,
}

/// Error returned by [`TcpListener::listen`]
///
/// Requires the `tcp-listener` feature.
#[cfg(feature = "tcp-listener")]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ListenError {
    /// The listener is already listening on a different address, or
    /// `bind_to_iface` was called while it is open.
    InvalidState,
    /// The listen port is zero.
    Unaddressable,
    /// Another TCP listener is bound to the identical address.
    InUse,
}

/// Error returned by [`TcpListener::accept`] and [`TcpSocket::accept`]
///
/// Requires the `tcp-listener` feature.
#[cfg(feature = "tcp-listener")]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AcceptError {
    /// The listener is not listening, or the socket is still open, so it can't be
    /// used for a new connection.
    InvalidState,
    /// The remote reset the connection during the handshake.
    ConnectionReset,
}

mod embedded_io_impls {
    use super::*;

    impl core::fmt::Display for ConnectError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("ConnectError")
        }
    }
    impl core::error::Error for ConnectError {}

    impl embedded_io_async::Error for ConnectError {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            match self {
                ConnectError::ConnectionReset => embedded_io_async::ErrorKind::ConnectionReset,
                ConnectError::TimedOut => embedded_io_async::ErrorKind::TimedOut,
                ConnectError::Unaddressable | ConnectError::NoFreePorts => embedded_io_async::ErrorKind::NotConnected,
                ConnectError::InvalidState | ConnectError::InUse => embedded_io_async::ErrorKind::Other,
            }
        }
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::ConnectionReset => f.write_str("ConnectionReset"),
            }
        }
    }
    impl core::error::Error for Error {}

    impl embedded_io_async::Error for Error {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            match self {
                Error::ConnectionReset => embedded_io_async::ErrorKind::ConnectionReset,
            }
        }
    }
}

#[cfg(feature = "tcp-listener")]
impl core::fmt::Display for AcceptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => f.write_str("InvalidState"),
            Self::ConnectionReset => f.write_str("ConnectionReset"),
        }
    }
}
#[cfg(feature = "tcp-listener")]
impl core::error::Error for AcceptError {}

#[cfg(feature = "tcp-listener")]
impl core::fmt::Display for ListenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => f.write_str("InvalidState"),
            Self::Unaddressable => f.write_str("Unaddressable"),
            Self::InUse => f.write_str("InUse"),
        }
    }
}
#[cfg(feature = "tcp-listener")]
impl core::error::Error for ListenError {}

// =======================

/// Shared low-level I/O helper used by `TcpSocket`, `TcpReader` and `TcpWriter`.
///
/// Fields and methods are private to this module, which makes them visible to
/// its child modules (`socket`, ...) but not to the rest of the crate.
#[derive(Copy, Clone)]
struct TcpIo<'d> {
    stack: Stack<'d>,
    handle: TcpHandle,
}

impl<'d> TcpIo<'d> {
    fn with<R>(&self, f: impl FnOnce(&mut tcp::TcpSocket<'_, 'd>) -> (R, WakeRunner)) -> R {
        self.stack.with(|i| f(&mut i.stack.tcp_socket(self.handle)))
    }

    /// Ready when `read` would not wait: there is data, or the receive half is
    /// closed and `read` returns at once.
    fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        self.with(|s| {
            (
                if s.can_recv() || !s.may_recv() {
                    Poll::Ready(())
                } else {
                    s.register_recv_waker(cx.waker());
                    Poll::Pending
                },
                NoWake,
            )
        })
    }

    fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, TryError<Error>> {
        self.with(|s| match s.recv_slice(buf) {
            // Reading into empty buffer
            Ok(0) if buf.is_empty() => (Ok(0), NoWake),
            // No data ready
            Ok(0) => (Err(TryError::WouldBlock), NoWake),
            // Data ready!
            Ok(n) => (Ok(n), Wake),
            // EOF
            Err(tcp::RecvError::Finished) => (Ok(0), NoWake),
            // Connection reset.
            Err(tcp::RecvError::InvalidState) => (Err(TryError::Other(Error::ConnectionReset)), NoWake),
        })
    }

    fn read<'s>(&'s mut self, buf: &'s mut [u8]) -> impl Future<Output = Result<usize, Error>> + 's {
        poll_fn(|cx| {
            // CAUTION: xarxa semantics around EOF are different to what you'd expect
            // from posix-like IO, so we have to tweak things here.
            self.with(|s| match s.recv_slice(buf) {
                // Reading into empty buffer
                Ok(0) if buf.is_empty() => {
                    // embedded_io_async::Read's contract is to not block if buf is empty. While
                    // this function is not a direct implementor of the trait method, we still don't
                    // want our future to never resolve.
                    (Poll::Ready(Ok(0)), NoWake)
                }
                // No data ready
                Ok(0) => {
                    s.register_recv_waker(cx.waker());
                    (Poll::Pending, NoWake)
                }
                // Data ready!
                Ok(n) => (Poll::Ready(Ok(n)), Wake),
                // EOF
                Err(tcp::RecvError::Finished) => (Poll::Ready(Ok(0)), NoWake),
                // Connection reset. TODO: this can also be timeouts etc, investigate.
                Err(tcp::RecvError::InvalidState) => (Poll::Ready(Err(Error::ConnectionReset)), NoWake),
            })
        })
    }

    /// Ready when `write` would not wait: there is buffer space, or the transmit
    /// half is closed and `write` fails at once.
    fn poll_write_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        self.with(|s| {
            (
                if s.can_send() || !s.may_send() {
                    Poll::Ready(())
                } else {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                },
                NoWake,
            )
        })
    }

    fn try_write(&mut self, buf: &[u8]) -> Result<usize, TryError<Error>> {
        self.with(|s| match s.send_slice(buf) {
            // Writing an empty buffer
            Ok(0) if buf.is_empty() => (Ok(0), NoWake),
            // Not ready to send (no space in the tx buffer)
            Ok(0) => (Err(TryError::WouldBlock), NoWake),
            // Some data sent
            Ok(n) => (Ok(n), Wake),
            // Connection reset.
            Err(tcp::SendError::InvalidState) => (Err(TryError::Other(Error::ConnectionReset)), NoWake),
        })
    }

    fn write<'s>(&'s mut self, buf: &'s [u8]) -> impl Future<Output = Result<usize, Error>> + 's {
        poll_fn(|cx| {
            self.with(|s| match s.send_slice(buf) {
                // Writing an empty buffer
                Ok(0) if buf.is_empty() => (Poll::Ready(Ok(0)), NoWake),
                // Not ready to send (no space in the tx buffer)
                Ok(0) => {
                    s.register_send_waker(cx.waker());
                    (Poll::Pending, NoWake)
                }
                // Some data sent
                Ok(n) => (Poll::Ready(Ok(n)), Wake),
                // Connection reset. TODO: this can also be timeouts etc, investigate.
                Err(tcp::SendError::InvalidState) => (Poll::Ready(Err(Error::ConnectionReset)), NoWake),
            })
        })
    }

    /// Run `f` on the send buffer, waking the runner if it queued anything.
    fn send_with<R>(
        s: &mut tcp::TcpSocket<'_, 'd>,
        f: impl FnOnce(&mut [u8]) -> (usize, R),
    ) -> (Result<R, Error>, WakeRunner) {
        match s.send(|buf| {
            let (n, r) = f(buf);
            (n, (n, r))
        }) {
            Ok((0, r)) => (Ok(r), NoWake),
            Ok((_, r)) => (Ok(r), Wake),
            Err(tcp::SendError::InvalidState) => (Err(Error::ConnectionReset), NoWake),
        }
    }

    /// Run `f` on the receive buffer, waking the runner if it took anything.
    fn recv_with<R>(
        s: &mut tcp::TcpSocket<'_, 'd>,
        f: impl FnOnce(&mut [u8]) -> (usize, R),
    ) -> (Result<R, Error>, WakeRunner) {
        match s.recv(|buf| {
            let (n, r) = f(buf);
            (n, (n, r))
        }) {
            Ok((0, r)) => (Ok(r), NoWake),
            Ok((_, r)) => (Ok(r), Wake),
            Err(tcp::RecvError::Finished) | Err(tcp::RecvError::InvalidState) => (Err(Error::ConnectionReset), NoWake),
        }
    }

    fn try_write_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, TryError<Error>> {
        self.with(|s| {
            if !s.can_send() {
                if s.may_send() {
                    (Err(TryError::WouldBlock), NoWake)
                } else {
                    (Err(TryError::Other(Error::ConnectionReset)), NoWake)
                }
            } else {
                let (r, wake) = Self::send_with(s, f);
                (r.map_err(TryError::Other), wake)
            }
        })
    }

    async fn write_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, Error> {
        let mut f = Some(f);
        poll_fn(move |cx| {
            self.with(|s| {
                if !s.can_send() {
                    if s.may_send() {
                        // socket buffer is full wait until it has at least one byte free
                        s.register_send_waker(cx.waker());
                        (Poll::Pending, NoWake)
                    } else {
                        // if we can't transmit because the transmit half of the duplex connection is closed then return an error
                        (Poll::Ready(Err(Error::ConnectionReset)), NoWake)
                    }
                } else {
                    // Connection reset. TODO: this can also be timeouts etc, investigate.
                    let (r, wake) = Self::send_with(s, unwrap!(f.take()));
                    (Poll::Ready(r), wake)
                }
            })
        })
        .await
    }

    fn try_read_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, TryError<Error>> {
        self.with(|s| {
            if !s.can_recv() {
                if s.may_recv() {
                    (Err(TryError::WouldBlock), NoWake)
                } else {
                    (Err(TryError::Other(Error::ConnectionReset)), NoWake)
                }
            } else {
                let (r, wake) = Self::recv_with(s, f);
                (r.map_err(TryError::Other), wake)
            }
        })
    }

    async fn read_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, Error> {
        let mut f = Some(f);
        poll_fn(move |cx| {
            self.with(|s| {
                if !s.can_recv() {
                    if s.may_recv() {
                        // socket buffer is empty wait until at least one byte has arrived
                        s.register_recv_waker(cx.waker());
                        (Poll::Pending, NoWake)
                    } else {
                        // if we can't receive because the receive half of the duplex connection is closed then return an error
                        (Poll::Ready(Err(Error::ConnectionReset)), NoWake)
                    }
                } else {
                    // Connection reset. TODO: this can also be timeouts etc, investigate.
                    let (r, wake) = Self::recv_with(s, unwrap!(f.take()));
                    (Poll::Ready(r), wake)
                }
            })
        })
        .await
    }

    /// Map the receive-half state to a peek result: `Ok(true)` if data can be
    /// peeked, `Ok(false)` if none has arrived yet, `Err(Ok(()))` on EOF.
    fn peek_state(s: &mut tcp::TcpSocket<'_, 'd>) -> Result<bool, Result<(), Error>> {
        match s.peek(0).map(|_| ()) {
            Err(tcp::RecvError::Finished) => Err(Ok(())),
            Err(tcp::RecvError::InvalidState) => Err(Err(Error::ConnectionReset)),
            Ok(()) => Ok(s.can_recv()),
        }
    }

    fn try_peek(&mut self, buf: &mut [u8]) -> Result<usize, TryError<Error>> {
        self.with(|s| {
            (
                match Self::peek_state(s) {
                    Err(Ok(())) => Ok(0),
                    Err(Err(e)) => Err(TryError::Other(e)),
                    Ok(_) if buf.is_empty() => Ok(0),
                    Ok(false) => Err(TryError::WouldBlock),
                    Ok(true) => Ok(unwrap!(s.peek_slice(buf))),
                },
                NoWake,
            )
        })
    }

    fn peek<'s>(&'s mut self, buf: &'s mut [u8]) -> impl Future<Output = Result<usize, Error>> + 's {
        poll_fn(|cx| {
            self.with(|s| {
                (
                    match Self::peek_state(s) {
                        Err(Ok(())) => Poll::Ready(Ok(0)),
                        Err(Err(e)) => Poll::Ready(Err(e)),
                        // Don't block if buf is empty, like `read`.
                        Ok(_) if buf.is_empty() => Poll::Ready(Ok(0)),
                        Ok(false) => {
                            s.register_recv_waker(cx.waker());
                            Poll::Pending
                        }
                        Ok(true) => Poll::Ready(Ok(unwrap!(s.peek_slice(buf)))),
                    },
                    NoWake,
                )
            })
        })
    }

    fn try_peek_with<R>(&mut self, f: impl FnOnce(&[u8]) -> R) -> Result<R, TryError<Error>> {
        self.with(|s| {
            (
                match Self::peek_state(s) {
                    Err(_) => Err(TryError::Other(Error::ConnectionReset)),
                    Ok(false) => Err(TryError::WouldBlock),
                    Ok(true) => Ok(f(unwrap!(s.peek(usize::MAX)))),
                },
                NoWake,
            )
        })
    }

    async fn peek_with<R>(&mut self, f: impl FnOnce(&[u8]) -> R) -> Result<R, Error> {
        let mut f = Some(f);
        poll_fn(move |cx| {
            self.with(|s| {
                (
                    match Self::peek_state(s) {
                        Err(_) => Poll::Ready(Err(Error::ConnectionReset)),
                        Ok(false) => {
                            s.register_recv_waker(cx.waker());
                            Poll::Pending
                        }
                        Ok(true) => Poll::Ready(Ok(unwrap!(f.take())(unwrap!(s.peek(usize::MAX))))),
                    },
                    NoWake,
                )
            })
        })
        .await
    }

    fn try_flush(&mut self) -> Result<(), TryError<Error>> {
        self.with(|s| {
            let data_pending = (s.send_queue() > 0) && s.state() != tcp::State::Closed;
            let fin_pending = matches!(
                s.state(),
                tcp::State::FinWait1 | tcp::State::Closing | tcp::State::LastAck
            );
            let rst_pending = s.state() == tcp::State::Closed && s.remote_addr().is_some();

            (
                if data_pending || fin_pending || rst_pending {
                    Err(TryError::WouldBlock)
                } else {
                    Ok(())
                },
                NoWake,
            )
        })
    }

    fn flush(&mut self) -> impl Future<Output = Result<(), Error>> + '_ {
        poll_fn(|cx| {
            self.with(|s| {
                let data_pending = (s.send_queue() > 0) && s.state() != tcp::State::Closed;
                let fin_pending = matches!(
                    s.state(),
                    tcp::State::FinWait1 | tcp::State::Closing | tcp::State::LastAck
                );
                let rst_pending = s.state() == tcp::State::Closed && s.remote_addr().is_some();

                // If there are outstanding send operations, register for wake up and wait
                // xarxa issues wake-ups when octets are dequeued from the send buffer
                if data_pending || fin_pending || rst_pending {
                    s.register_send_waker(cx.waker());
                    (Poll::Pending, NoWake)
                // No outstanding sends, socket is flushed
                } else {
                    (Poll::Ready(Ok(())), NoWake)
                }
            })
        })
    }

    fn recv_capacity(&self) -> usize {
        self.with(|s| (s.recv_capacity(), NoWake))
    }

    fn send_capacity(&self) -> usize {
        self.with(|s| (s.send_capacity(), NoWake))
    }

    fn send_queue(&self) -> usize {
        self.with(|s| (s.send_queue(), NoWake))
    }

    fn recv_queue(&self) -> usize {
        self.with(|s| (s.recv_queue(), NoWake))
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write;

    use embedded_io_async::{Error as _, ErrorKind};

    use super::*;

    /// Tiny fixed-size `fmt::Write` sink so tests work without `std`/`alloc`.
    struct Buf {
        data: [u8; 32],
        len: usize,
    }

    impl Buf {
        fn render(v: &impl core::fmt::Display) -> Self {
            let mut b = Buf { data: [0; 32], len: 0 };
            write!(b, "{}", v).unwrap();
            b
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.data[..self.len]).unwrap()
        }
    }

    impl Write for Buf {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            if self.len + bytes.len() > self.data.len() {
                return Err(core::fmt::Error);
            }
            self.data[self.len..self.len + bytes.len()].copy_from_slice(bytes);
            self.len += bytes.len();
            Ok(())
        }
    }

    fn assert_core_error<E: core::error::Error + Copy + Eq>() {}

    #[test]
    fn error_display() {
        assert_eq!(Buf::render(&Error::ConnectionReset).as_str(), "ConnectionReset");
        assert_eq!(Buf::render(&ConnectError::TimedOut).as_str(), "ConnectError");
    }

    #[test]
    fn error_kind() {
        assert_eq!(Error::ConnectionReset.kind(), ErrorKind::ConnectionReset);
    }

    #[test]
    fn connect_error_kind_mapping() {
        let table = [
            (ConnectError::ConnectionReset, ErrorKind::ConnectionReset),
            (ConnectError::TimedOut, ErrorKind::TimedOut),
            (ConnectError::Unaddressable, ErrorKind::NotConnected),
            (ConnectError::NoFreePorts, ErrorKind::NotConnected),
            (ConnectError::InvalidState, ErrorKind::Other),
            (ConnectError::InUse, ErrorKind::Other),
        ];
        for (err, kind) in table {
            assert_eq!(err.kind(), kind, "wrong kind for {:?}", err);
        }
    }

    #[test]
    fn errors_are_copy_eq_and_core_error() {
        assert_core_error::<Error>();
        assert_core_error::<ConnectError>();

        let a = ConnectError::InUse;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(ConnectError::InUse, ConnectError::InvalidState);
    }

    #[cfg(feature = "tcp-listener")]
    #[test]
    fn listener_errors_display() {
        assert_eq!(Buf::render(&AcceptError::InvalidState).as_str(), "InvalidState");
        assert_eq!(Buf::render(&AcceptError::ConnectionReset).as_str(), "ConnectionReset");
        assert_eq!(Buf::render(&ListenError::InvalidState).as_str(), "InvalidState");
        assert_eq!(Buf::render(&ListenError::Unaddressable).as_str(), "Unaddressable");
        assert_eq!(Buf::render(&ListenError::InUse).as_str(), "InUse");
    }

    #[cfg(feature = "tcp-listener")]
    #[test]
    fn listener_errors_traits() {
        assert_core_error::<AcceptError>();
        assert_core_error::<ListenError>();
        assert_ne!(ListenError::InUse, ListenError::Unaddressable);
        assert_ne!(AcceptError::InvalidState, AcceptError::ConnectionReset);
    }
}