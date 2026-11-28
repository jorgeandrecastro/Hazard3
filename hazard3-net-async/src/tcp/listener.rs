// SPDX-License-Identifier: MIT OR Apache-2.0

//! TCP listener. Requires the `tcp-listener` feature (gated in the parent module).

use core::future::{Future, poll_fn};
use core::task::Poll;

use hazard3_net::error::Full;
use hazard3_net::tcp::{self, TcpListenerHandle};
use hazard3_net::wire::ListenSocketAddr;

#[cfg(feature = "iface-bind")]
use super::IfaceHandle;
use super::{AcceptError, AcceptToken, ListenError};
use crate::{NoWake, Stack, WakeRunner};

/// A TCP listener.
///
/// Use a [`TcpListener`] to accept incoming TCP connections.
///
/// A listener can be bound to a port and optionally an address.
/// It receives all incoming connection attempts and queues them.
/// Calling [`accept`](TcpListener::accept) pops an attempt from the queue as
/// an [`AcceptToken`]. Pass it to [`TcpSocket::accept`](super::TcpSocket::accept)
/// to set up a socket for it, possibly in another task.
///
/// Connection attempts (SYN packets) are not answered (with a SYN|ACK packet)
/// until a socket accepts them. The queue depth is set by the
/// `tcp-listener-backlog-N` feature of `xarxa`.
pub struct TcpListener<'d> {
    stack: Stack<'d>,
    handle: TcpListenerHandle,
}

impl<'d> TcpListener<'d> {
    /// Create a new TCP listener on the given stack.
    ///
    /// The listener starts closed. Call [`listen`](Self::listen) to start listening.
    ///
    /// # Errors
    /// - `Full`: if the stack has no room for another listener. Only possible
    ///   without the `alloc` feature, where the limit is set by the
    ///   `tcp-listener-count-N` feature of `xarxa`.
    pub fn new(stack: Stack<'d>) -> Result<Self, Full> {
        let handle = stack.with(|i| (i.stack.add_tcp_listener(), NoWake))?;

        Ok(Self { stack, handle })
    }

    fn with<R>(&self, f: impl FnOnce(&mut tcp::TcpListener<'_>) -> (R, WakeRunner)) -> R {
        self.stack.with(|i| f(&mut i.stack.tcp_listener(self.handle)))
    }

    /// Bind the listener to an interface, or unbind it with `None`.
    ///
    /// A listener bound to an interface only accepts connection attempts that arrive on that interface.
    ///
    /// When accepting a connection, the sockets inherit the binding.
    ///
    /// The listener must be closed. The binding is kept across
    /// [`close`](Self::close).
    ///
    /// Two listeners on the same address can coexist if they are bound to different interfaces,
    /// or if one is not bound. In the latter case, the bound listener "wins" for incoming
    /// connections coming from the bound interface.
    ///
    /// # Errors
    /// - `InvalidState`: if the listener is open.
    #[cfg(feature = "iface-bind")]
    pub fn bind_to_iface(&mut self, iface: Option<IfaceHandle>) -> Result<(), ListenError> {
        match self.with(|l| (l.bind_to_iface(iface), NoWake)) {
            Ok(()) => Ok(()),
            Err(tcp::ListenError::InvalidState) => Err(ListenError::InvalidState),
            Err(_) => unreachable!(),
        }
    }

    /// Return the interface the listener is bound to, or `None`.
    ///
    /// See [`bind_to_iface`](Self::bind_to_iface).
    #[cfg(feature = "iface-bind")]
    pub fn bound_iface(&self) -> Option<IfaceHandle> {
        self.with(|l| (l.bound_iface(), NoWake))
    }

    /// Start listening on the given local address.
    ///
    /// # Errors
    /// - `Unaddressable`: if the port is zero.
    /// - `InvalidState`: if the listener is already listening (unless it is
    ///   listening on this same address, which is a no-op).
    /// - `InUse`: if another listener is bound to an identical address.
    ///   Listeners on the same port with *different* specificity (one wildcard,
    ///   one per-version, one per-address) may coexist, and so may listeners on
    ///   identical addresses bound to different interfaces.
    pub fn listen(&mut self, local: impl Into<ListenSocketAddr>) -> Result<(), ListenError> {
        match self.with(|l| (l.listen(local), NoWake)) {
            Ok(()) => Ok(()),
            Err(tcp::ListenError::InvalidState) => Err(ListenError::InvalidState),
            Err(tcp::ListenError::Unaddressable) => Err(ListenError::Unaddressable),
            Err(tcp::ListenError::InUse) => Err(ListenError::InUse),
        }
    }

    /// Stop listening, dropping all queued SYNs.
    ///
    /// The dropped SYNs are not reset. The clients' retransmissions are
    /// answered with an RST once the listener is gone.
    pub fn close(&mut self) {
        self.with(|l| (l.close(), NoWake))
    }

    /// Whether the listener is listening.
    pub fn is_open(&self) -> bool {
        self.with(|l| (l.is_open(), NoWake))
    }

    /// Return the listened address. The address is the filter the listen scoped
    /// the listener to. A zero port means the listener is closed.
    pub fn local_addr(&self) -> ListenSocketAddr {
        self.with(|l| (l.local_addr(), NoWake))
    }

    /// Whether a connection attempt is waiting to be [`accept`](Self::accept)ed.
    pub fn can_accept(&self) -> bool {
        self.with(|l| (l.can_accept(), NoWake))
    }

    /// Wait until a connection attempt is waiting to be accepted, or the listener
    /// is not listening.
    ///
    /// This is the equivalent of [accept](#method.accept), without accepting the connection.
    pub fn wait_accept_ready(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(move |cx| {
            self.with(|l| {
                (
                    if l.can_accept() || !l.is_open() {
                        Poll::Ready(())
                    } else {
                        l.register_accept_waker(cx.waker());
                        Poll::Pending
                    },
                    NoWake,
                )
            })
        })
    }

    /// Accept a queued connection attempt, waiting if none is queued yet.
    ///
    /// Returns an [`AcceptToken`].
    ///
    /// Pass it to [`TcpSocket::accept`](super::TcpSocket::accept) to set up a socket for the
    /// incoming connection, possibly in another task.
    ///
    /// Dropping the token forgets the attempt. The client retransmits its SYN,
    /// which queues the attempt on the listener again.
    ///
    /// # Errors
    /// - `InvalidState`: if the listener is not listening.
    pub async fn accept(&mut self) -> Result<AcceptToken, AcceptError> {
        poll_fn(|cx| {
            self.with(|l| {
                if !l.is_open() {
                    return (Poll::Ready(Err(AcceptError::InvalidState)), NoWake);
                }
                (
                    match l.accept() {
                        Some(token) => Poll::Ready(Ok(token)),
                        None => {
                            l.register_accept_waker(cx.waker());
                            Poll::Pending
                        }
                    },
                    NoWake,
                )
            })
        })
        .await
    }
}

impl Drop for TcpListener<'_> {
    fn drop(&mut self) {
        self.stack.with(|i| (i.stack.remove_tcp_listener(self.handle), NoWake));
    }
}

#[cfg(test)]
mod tests {
    //! Compile-time checks of the public listener API.
    //! Runtime tests need a `Stack` with a mock driver.

    use super::*;

    fn assert_output<T, F: Future<Output = T>>(_: F) {}

    #[allow(dead_code)]
    fn listener_api(l: &mut TcpListener<'_>) {
        let _: Result<(), ListenError> = l.listen(ListenSocketAddr::UNSPECIFIED);
        let _: ListenSocketAddr = l.local_addr();
        let _: bool = l.is_open();
        let _: bool = l.can_accept();
        l.close();

        assert_output::<(), _>(l.wait_accept_ready());
    }

    #[allow(dead_code)]
    fn accept_api(l: &mut TcpListener<'_>) {
        assert_output::<Result<AcceptToken, AcceptError>, _>(l.accept());
    }

    #[cfg(feature = "iface-bind")]
    #[allow(dead_code)]
    fn bind_api(l: &mut TcpListener<'_>) {
        let _: Result<(), ListenError> = l.bind_to_iface(None);
        let _: Option<IfaceHandle> = l.bound_iface();
    }

    #[allow(dead_code)]
    fn new_api(stack: Stack<'_>) {
        let _: Result<TcpListener<'_>, Full> = TcpListener::new(stack);
    }
}