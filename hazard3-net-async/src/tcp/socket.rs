// SPDX-License-Identifier: MIT OR Apache-2.0

//! TCP socket: construction, connection management, configuration and state.
//!
//! The data-transfer half (`read`/`write`/`peek`/`flush`, [`TcpReader`],
//! [`TcpWriter`], `embedded-io` impls) lives in the `io` submodule.

use core::future::poll_fn;
use core::marker::PhantomData;
use core::mem;
use core::task::Poll;

use embassy_time::Duration;
use xarxa::error::{Full, InvalidHopLimit};
use xarxa::tcp;
use xarxa::wire::{ListenSocketAddr, SocketAddr};

#[cfg(feature = "iface-bind")]
use super::IfaceHandle;
#[cfg(feature = "tcp-listener")]
use super::{AcceptError, AcceptToken};
use super::{ConnectError, State, TcpIo};
use crate::time::{duration_from_xarxa, duration_to_xarxa};
use crate::{NoWake, Stack, TryError, wake_if, wake_if_ok};

mod io;

/// A Transmission Control Protocol socket.
///
#[cfg_attr(
    not(feature = "tcp-listener"),
    doc = "A TCP socket represents a single connection (connecting or connected): its",
    doc = "4-tuple is fully set from the start, by [`connect`](Self::connect)."
)]
#[cfg_attr(
    feature = "tcp-listener",
    doc = "A TCP socket represents a single connection (connecting or connected): its",
    doc = "4-tuple is fully set from the start, by [`connect`](Self::connect) or by",
    doc = "[`accept`](Self::accept). Passive open lives in [`TcpListener`](super::TcpListener)."
)]
///
/// `'a` is the lifetime of the socket's buffers, `'d` the lifetime of the
/// stack.
pub struct TcpSocket<'a, 'd> {
    io: TcpIo<'d>,
    bufs: PhantomData<&'a mut [u8]>,
}

/// The reader half of a TCP socket.
pub struct TcpReader<'a, 'd> {
    io: TcpIo<'d>,
    bufs: PhantomData<&'a mut [u8]>,
}

/// The writer half of a TCP socket.
pub struct TcpWriter<'a, 'd> {
    io: TcpIo<'d>,
    bufs: PhantomData<&'a mut [u8]>,
}

impl<'a, 'd> TcpSocket<'a, 'd> {
    /// Create a new TCP socket on the given stack, with the given buffers.
    ///
    /// # Errors
    /// - `Full`: if the stack has no room for another TCP socket. Only possible
    ///   without the `alloc` feature, where the limit is set by the
    ///   `tcp-socket-count-N` feature of `xarxa`.
    pub fn new(stack: Stack<'d>, rx_buffer: &'a mut [u8], tx_buffer: &'a mut [u8]) -> Result<Self, Full> {
        let handle = stack.with(|i| {
            let rx_buffer: &'d mut [u8] = unsafe { mem::transmute(rx_buffer) };
            let tx_buffer: &'d mut [u8] = unsafe { mem::transmute(tx_buffer) };
            (i.stack.add_tcp_socket_with_bufs(rx_buffer, tx_buffer), NoWake)
        })?;

        Ok(Self {
            io: TcpIo { stack, handle },
            bufs: PhantomData,
        })
    }

    /// Bind the socket to an interface, or unbind it with `None`.
    ///
    /// A socket bound to an interface only sends and receives packets on it:
    /// - Destinations must be on-link on that iface, or have a route through it.
    /// - Broadcast and multicast destinations go out on that iface only
    /// - Local addresses will be picked from that iface only.
    ///
    /// The socket must be closed (see [`is_open`](Self::is_open)). The binding
    /// is kept across connections, so a reused socket stays bound to its interface.
    /// Accepting a connection into the socket overwrites it with the listener's binding.
    ///
    /// Two sockets with otherwise identical tuples may coexist if they are
    /// bound to different interfaces.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is open.
    #[cfg(feature = "iface-bind")]
    pub fn bind_to_iface(&mut self, iface: Option<IfaceHandle>) -> Result<(), ConnectError> {
        match self.io.with(|s| (s.bind_to_iface(iface), NoWake)) {
            Ok(()) => Ok(()),
            Err(tcp::ConnectError::InvalidState) => Err(ConnectError::InvalidState),
            Err(_) => unreachable!(),
        }
    }

    /// Return the interface the socket is bound to, or `None`.
    ///
    /// See [`bind_to_iface`](Self::bind_to_iface).
    #[cfg(feature = "iface-bind")]
    pub fn bound_iface(&self) -> Option<IfaceHandle> {
        self.io.with(|s| (s.bound_iface(), NoWake))
    }

    fn start_connect(&mut self, remote: impl Into<SocketAddr>) -> Result<(), ConnectError> {
        match self
            .io
            .with(|s| wake_if_ok(s.connect(remote, ListenSocketAddr::UNSPECIFIED)))
        {
            Ok(()) => Ok(()),
            Err(tcp::ConnectError::InvalidState) => Err(ConnectError::InvalidState),
            Err(tcp::ConnectError::Unaddressable) => Err(ConnectError::Unaddressable),
            Err(tcp::ConnectError::NoFreePorts) => Err(ConnectError::NoFreePorts),
            Err(tcp::ConnectError::InUse) => Err(ConnectError::InUse),
        }
    }

    /// Connect to a given remote address, and wait until the connection is established.
    ///
    /// The local port is an ephemeral port (a free port in the 49152..=65535
    /// range, picked at a random starting point), and the local address is
    /// selected by the stack from the remote address.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is open.
    /// - `Unaddressable`: if the remote port is zero, the remote address is
    ///   unspecified, there is no route to the remote address, or the interface
    ///   the route goes out of has no address to send from.
    /// - `NoFreePorts`: if the ephemeral range is exhausted.
    /// - `InUse`: if another TCP socket already holds the identical 4-tuple.
    /// - `ConnectionReset`: if the connection is reset or aborted during the handshake.
    pub async fn connect(&mut self, remote: impl Into<SocketAddr>) -> Result<(), ConnectError> {
        self.start_connect(remote)?;

        poll_fn(|cx| {
            self.io.with(|s| {
                (
                    match s.state() {
                        tcp::State::Closed | tcp::State::TimeWait => Poll::Ready(Err(ConnectError::ConnectionReset)),
                        tcp::State::SynSent | tcp::State::SynReceived => {
                            s.register_send_waker(cx.waker());
                            Poll::Pending
                        }
                        _ => Poll::Ready(Ok(())),
                    },
                    NoWake,
                )
            })
        })
        .await
    }

    /// Accept a connection attempt into this socket, and wait until its handshake completes.
    ///
    /// The token comes from [`TcpListener::accept`](super::TcpListener::accept).
    ///
    /// The socket's interface binding (see `bind_to_iface`)
    /// is set to the listener's binding. Other configuration (hop limit,
    /// timeout, keep-alive, Nagle, ACK delay) is left unchanged.
    ///
    /// If the returned future is dropped before it completes, the connection attempt is reset.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not closed.
    /// - `ConnectionReset`: if the remote resets the connection during the handshake.
    #[cfg(feature = "tcp-listener")]
    pub async fn accept(&mut self, token: AcceptToken) -> Result<(), AcceptError> {
        self.io.with(|s| wake_if_ok(s.accept(token))).map_err(|e| match e {
            tcp::AcceptError::InvalidState => AcceptError::InvalidState,
        })?;

        poll_fn(|cx| {
            self.io.with(|s| {
                (
                    match s.state() {
                        tcp::State::Closed | tcp::State::TimeWait => Poll::Ready(Err(AcceptError::ConnectionReset)),
                        tcp::State::SynReceived => {
                            s.register_send_waker(cx.waker());
                            Poll::Pending
                        }
                        _ => Poll::Ready(Ok(())),
                    },
                    NoWake,
                )
            })
        })
        .await
    }

    /// Connect to a remote host.
    ///
    /// This method will not wait for the connection to be established.
    ///
    /// If the socket is not already connecting, this method will initiate the connection
    /// and return `Err(TryError::WouldBlock)`. While the connection is being established,
    /// it will continue to return `Err(TryError::WouldBlock)`.
    ///
    /// Once the connection is successfully established and ready to send/receive data,
    /// this method will return `Ok(())`.
    pub fn try_connect(&mut self, remote: impl Into<SocketAddr>) -> Result<(), TryError<ConnectError>> {
        match self.state() {
            tcp::State::Closed | tcp::State::TimeWait => match self.start_connect(remote) {
                Ok(()) => Err(TryError::WouldBlock),
                Err(e) => Err(TryError::Other(e)),
            },
            tcp::State::SynSent | tcp::State::SynReceived => Err(TryError::WouldBlock),
            _ => Ok(()),
        }
    }

    /// Set the timeout duration.
    ///
    /// A socket with a timeout duration set will abort the connection if either of the following
    /// occurs:
    ///
    ///   * After a [connect](#method.connect) call, the remote peer does not respond within
    ///     the specified duration;
    ///   * After establishing a connection, there is data in the transmit buffer and the remote
    ///     peer exceeds the specified duration between any two packets it sends;
    ///   * After enabling [keep-alive](#method.set_keep_alive), the remote peer exceeds
    ///     the specified duration between any two packets it sends.
    pub fn set_timeout(&mut self, duration: Option<Duration>) {
        self.io.with(|s| {
            (
                s.set_timeout(duration.map(duration_to_xarxa)),
                wake_if(duration.is_some()),
            )
        })
    }

    /// Return the timeout duration.
    ///
    /// See also the [set_timeout](#method.set_timeout) method.
    pub fn timeout(&self) -> Option<Duration> {
        self.io.with(|s| (s.timeout().map(duration_from_xarxa), NoWake))
    }

    /// Set the ACK delay duration.
    ///
    /// By default, the ACK delay is set to 10ms.
    pub fn set_ack_delay(&mut self, duration: Option<Duration>) {
        self.io
            .with(|s| (s.set_ack_delay(duration.map(duration_to_xarxa)), NoWake))
    }

    /// Return the ACK delay duration.
    ///
    /// See also the [set_ack_delay](#method.set_ack_delay) method.
    pub fn ack_delay(&self) -> Option<Duration> {
        self.io.with(|s| (s.ack_delay().map(duration_from_xarxa), NoWake))
    }

    /// Set the keep-alive interval.
    ///
    /// An idle socket with a keep-alive interval set will transmit a "keep-alive ACK" packet
    /// every time it receives no communication during that interval. As a result, three things
    /// may happen:
    ///
    ///   * The remote peer is fine and answers with an ACK packet.
    ///   * The remote peer has rebooted and answers with an RST packet.
    ///   * The remote peer has crashed and does not answer.
    ///
    /// The keep-alive functionality together with the timeout functionality allows to react
    /// to these error conditions.
    pub fn set_keep_alive(&mut self, interval: Option<Duration>) {
        self.io.with(|s| {
            (
                s.set_keep_alive(interval.map(duration_to_xarxa)),
                wake_if(interval.is_some()),
            )
        })
    }

    /// Return the keep-alive interval.
    ///
    /// See also the [set_keep_alive](#method.set_keep_alive) method.
    pub fn keep_alive(&self) -> Option<Duration> {
        self.io.with(|s| (s.keep_alive().map(duration_from_xarxa), NoWake))
    }

    /// Set the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    ///
    /// A socket without an explicitly set hop limit value uses the default [IANA recommended]
    /// value (64).
    ///
    /// # Errors
    /// - `InvalidHopLimit`: if the hop limit is `Some(0)`. A host must not send a
    ///   packet with a hop limit of zero ([RFC 1122 § 3.2.1.7]). The socket is
    ///   left unchanged.
    ///
    /// [IANA recommended]: https://www.iana.org/assignments/ip-parameters/ip-parameters.xhtml
    /// [RFC 1122 § 3.2.1.7]: https://tools.ietf.org/html/rfc1122#section-3.2.1.7
    pub fn set_hop_limit(&mut self, hop_limit: Option<u8>) -> Result<(), InvalidHopLimit> {
        self.io.with(|s| (s.set_hop_limit(hop_limit), NoWake))
    }

    /// Return the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    ///
    /// See also the [set_hop_limit](#method.set_hop_limit) method
    pub fn hop_limit(&self) -> Option<u8> {
        self.io.with(|s| (s.hop_limit(), NoWake))
    }

    /// Enable or disable Nagle's Algorithm.
    ///
    /// Also known as "tinygram prevention". By default, it is enabled.
    /// Disabling it is equivalent to Linux's TCP_NODELAY flag.
    ///
    /// When enabled, Nagle's Algorithm prevents sending segments smaller than MSS if
    /// there is data in flight (sent but not acknowledged). In other words, it ensures
    /// at most only one segment smaller than MSS is in flight at a time.
    ///
    /// It ensures better network utilization by preventing sending many very small packets,
    /// at the cost of increased latency in some situations, particularly when the remote peer
    /// has ACK delay enabled.
    pub fn set_nagle_enabled(&mut self, enabled: bool) {
        self.io.with(|s| (s.set_nagle_enabled(enabled), wake_if(!enabled)))
    }

    /// Return whether Nagle's Algorithm is enabled.
    ///
    /// See also the [set_nagle_enabled](#method.set_nagle_enabled) method.
    pub fn nagle_enabled(&self) -> bool {
        self.io.with(|s| (s.nagle_enabled(), NoWake))
    }

    /// Return the local address, or None if not connected.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.io.with(|s| (s.local_addr(), NoWake))
    }

    /// Return the remote address, or None if not connected.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.io.with(|s| (s.remote_addr(), NoWake))
    }

    /// Return the connection state, in terms of the TCP state machine.
    pub fn state(&self) -> State {
        self.io.with(|s| (s.state(), NoWake))
    }

    /// Return whether the socket is open.
    ///
    /// This function returns true if the socket will process incoming or dispatch outgoing
    /// packets. Note that this does not mean that it is possible to send or receive data through
    /// the socket; for that, use [can_send](#method.can_send) or [can_recv](#method.can_recv).
    ///
    /// In terms of the TCP state machine, the socket must not be in the `CLOSED`
    /// or `TIME-WAIT` states.
    pub fn is_open(&self) -> bool {
        self.io.with(|s| (s.is_open(), NoWake))
    }

    /// Return whether a connection is active.
    ///
    /// This function returns true if the socket is actively exchanging packets with
    /// a remote peer. Note that this does not mean that it is possible to send or receive
    /// data through the socket; for that, use [can_send](#method.can_send) or
    /// [can_recv](#method.can_recv).
    ///
    /// If a connection is established, [abort](#method.abort) will send a reset to
    /// the remote peer.
    ///
    /// In terms of the TCP state machine, the socket must not be in the `CLOSED`
    /// or `TIME-WAIT` state.
    pub fn is_active(&self) -> bool {
        self.io.with(|s| (s.is_active(), NoWake))
    }

    /// Take the pending ICMP error, if one has been reported against this
    /// connection.
    #[cfg(feature = "icmp-errors")]
    pub fn take_icmp_error(&mut self) -> Option<crate::error::IcmpError> {
        self.io.with(|s| (s.take_icmp_error(), NoWake))
    }

    /// Close the transmit half of the full-duplex connection.
    ///
    /// Data that has been written to the socket and not yet sent (or not yet ACKed) will
    /// still be sent. The last segment of the pending to send data is sent with the FIN flag set.
    ///
    /// Note that there is no corresponding function for the receive half of the full-duplex
    /// connection; only the remote end can close it. If you no longer wish to receive any
    /// data and would like to reuse the socket right away, use [abort](#method.abort).
    pub fn close(&mut self) {
        self.io.with(|s| {
            let state = s.state();
            s.close();
            ((), wake_if(s.state() != state))
        })
    }

    /// Aborts the connection, if any.
    ///
    /// This function instantly closes the socket. One reset packet will be sent to the remote
    /// peer.
    ///
    /// In terms of the TCP state machine, the socket may be in any state and is moved to
    /// the `CLOSED` state.
    ///
    /// Note that the TCP RST packet is not sent immediately - if the `TcpSocket` is dropped too soon
    /// the remote host may not know the connection has been closed.
    /// `abort()` callers should wait for a [`flush()`](TcpSocket::flush) call to complete before
    /// dropping or reusing the socket.
    pub fn abort(&mut self) {
        self.io.with(|s| {
            let state = s.state();
            s.abort();
            ((), wake_if(s.state() != state))
        })
    }

    /// Return whether the transmit half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to send data and have it arrive
    /// to the remote peer. However, it does not make any guarantees about the state
    /// of the transmit buffer, and even if it returns true, [write](#method.write) may
    /// not be able to enqueue any octets.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED` or
    /// `CLOSE-WAIT` state.
    pub fn may_send(&self) -> bool {
        self.io.with(|s| (s.may_send(), NoWake))
    }

    /// Check whether the transmit half of the full-duplex connection is open
    /// (see [may_send](#method.may_send)), and the transmit buffer is not full.
    pub fn can_send(&self) -> bool {
        self.io.with(|s| (s.can_send(), NoWake))
    }

    /// Return whether the receive half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to receive data from the remote peer.
    /// It will return true while there is data in the receive buffer, and if there isn't,
    /// as long as the remote peer has not closed the connection.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED`,
    /// `FIN-WAIT-1`, or `FIN-WAIT-2` state, or have data in the receive buffer instead.
    pub fn may_recv(&self) -> bool {
        self.io.with(|s| (s.may_recv(), NoWake))
    }

    /// Check whether the receive buffer is not empty.
    pub fn can_recv(&self) -> bool {
        self.io.with(|s| (s.can_recv(), NoWake))
    }
}

impl Drop for TcpSocket<'_, '_> {
    fn drop(&mut self) {
        self.io
            .stack
            .with(|i| (i.stack.remove_tcp_socket(self.io.handle), NoWake));
    }
}

// `'a` (the buffers) is covariant; `'d` (the stack) is invariant.
fn _assert_covariant<'a, 'b: 'a, 'd>(x: TcpSocket<'b, 'd>) -> TcpSocket<'a, 'd> {
    x
}
fn _assert_covariant_reader<'a, 'b: 'a, 'd>(x: TcpReader<'b, 'd>) -> TcpReader<'a, 'd> {
    x
}
fn _assert_covariant_writer<'a, 'b: 'a, 'd>(x: TcpWriter<'b, 'd>) -> TcpWriter<'a, 'd> {
    x
}

#[cfg(test)]
mod tests {
    //! Compile-time checks of the connection/configuration API. They guard
    //! against accidental signature changes. Runtime tests need a `Stack`
    //! with a mock driver.

    use core::future::Future;

    use super::*;

    fn assert_output<T, F: Future<Output = T>>(_: F) {}

    #[allow(dead_code)]
    fn new_api<'a, 'd>(stack: Stack<'d>, rx: &'a mut [u8], tx: &'a mut [u8]) {
        let _: Result<TcpSocket<'a, 'd>, Full> = TcpSocket::new(stack, rx, tx);
    }

    #[allow(dead_code)]
    fn connect_api<A: Into<SocketAddr>>(s: &mut TcpSocket<'_, '_>, addr: A) {
        assert_output::<Result<(), ConnectError>, _>(s.connect(addr));
    }

    #[allow(dead_code)]
    fn try_connect_api<A: Into<SocketAddr>>(s: &mut TcpSocket<'_, '_>, addr: A) {
        let _: Result<(), TryError<ConnectError>> = s.try_connect(addr);
    }

    #[cfg(feature = "tcp-listener")]
    #[allow(dead_code)]
    fn accept_api(s: &mut TcpSocket<'_, '_>, token: AcceptToken) {
        assert_output::<Result<(), AcceptError>, _>(s.accept(token));
    }

    #[cfg(feature = "iface-bind")]
    #[allow(dead_code)]
    fn bind_api(s: &mut TcpSocket<'_, '_>) {
        let _: Result<(), ConnectError> = s.bind_to_iface(None);
        let _: Option<IfaceHandle> = s.bound_iface();
    }

    #[allow(dead_code)]
    fn config_and_state_api(s: &mut TcpSocket<'_, '_>) {
        let _: State = s.state();
        let _: Option<SocketAddr> = s.local_addr();
        let _: Option<SocketAddr> = s.remote_addr();
        let _: Option<Duration> = s.timeout();
        let _: Option<Duration> = s.keep_alive();
        let _: Option<Duration> = s.ack_delay();
        let _: Option<u8> = s.hop_limit();
        let _: bool = s.nagle_enabled();
        let _: bool = s.is_open();
        let _: bool = s.is_active();
        let _: bool = s.may_send();
        let _: bool = s.can_send();
        let _: bool = s.may_recv();
        let _: bool = s.can_recv();

        s.set_timeout(None);
        s.set_keep_alive(None);
        s.set_ack_delay(None);
        s.set_nagle_enabled(false);
        let _: Result<(), InvalidHopLimit> = s.set_hop_limit(Some(64));
        s.close();
        s.abort();
    }
}