//! Socket lifecycle and configuration: creation, binding, addresses, state, options.

use hazard3_net::driver::PacketBuf;
use hazard3_net::error::InvalidHopLimit;
use hazard3_net::udp;
use hazard3_net::wire::ListenSocketAddr;

use super::{BindError, UdpSocket};
#[cfg(feature = "iface-bind")]
use super::IfaceHandle;
use crate::error::Full;
use crate::{NoWake, Stack, WakeRunner, wake_if};

impl<'d> UdpSocket<'d> {
    /// Create a new UDP socket using the provided stack.
    ///
    /// Errors:
    /// - `Full` if the stack has no room for another UDP socket. The limit is set
    ///   by the `udp-socket-count-N` feature of `xarxa`.
    pub fn new(stack: Stack<'d>) -> Result<Self, Full> {
        let handle = stack.with(|i| (i.stack.add_udp_socket(), NoWake))?;

        Ok(Self { stack, handle })
    }

    /// Bind the socket.
    ///
    /// This method opens the socket and configures which packets it will
    /// send and receive. It is equivalent to `bind()` and/or `connect()` in the
    /// BSD / Linux socket API.
    ///
    /// # Local address
    ///
    /// - `None`: the socket sends/receives packets with any local address. It receives packets destined to unicast, multicast and broadcast addresses.
    /// - `Some(V4(UNSPECIFIED))`: same as `None`, but IPv4 packets only.
    /// - `Some(V6(UNSPECIFIED))`: same as `None`, but IPv6 packets only.
    /// - `Some(_)`: the socket sends/receives packets from/to the given local address only.
    ///   - If unicast, the stack checks the address is ours, else returns `Unaddressable`.
    ///   - Multicast and broadcast addresses are allowed, but then the socket can receive only, not send.
    ///
    /// # Local port
    ///
    /// - `0`: the stack allocates an unused ephemeral port. You can retrieve it with [`local_addr`](Self::local_addr).
    /// - non-zero: the given port is used.
    ///
    /// The socket only receives packets to that port and sends from that port. A UDP socket *must* be bound to at least a port, there's no way to make a UDP socket listen on all ports.
    ///
    /// # Remote address
    ///
    /// - `None`: the socket sends/receives packets with any remote address.
    /// - `Some(V4(UNSPECIFIED))`: same as `None`, but IPv4 packets only.
    /// - `Some(V6(UNSPECIFIED))`: same as `None`, but IPv6 packets only.
    /// - `Some(_)`: the socket sends/receives packets to/from the given remote address only. Multicast and broadcast addresses are allowed, but then the socket can send only, not receive.
    ///
    /// # Remote port
    ///
    /// - `0`: the socket sends/receives packets to/from any remote port.
    /// - non-zero: the socket sends/receives packets to/from the given port only.
    ///
    /// Overlapping bindings between sockets are allowed as long as they're
    /// not identical. For example you can bind a socket to `*:53` and another to
    /// `1.2.3.4:53`, but you can't bind two sockets to `*:53`. If a packet
    /// matches multiple sockets, the one with the most specific binding wins.
    /// Packets are not duplicated, only the winning socket will receive it.
    ///
    /// Multicast groups are not joined automatically, you must call [`Iface::join_multicast_group`](crate::iface::Iface::join_multicast_group) yourself.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is already bound (see
    ///   [is_open](#method.is_open)).
    /// - `InUse`: on an identical bind.
    /// - `NoFreePorts`: if the ephemeral range is exhausted.
    /// - `Unaddressable`: on an address family mismatch, if the local address is
    ///   not ours, or if no local address is available for the given
    ///   remote.
    pub fn bind(
        &mut self,
        local: impl Into<ListenSocketAddr>,
        remote: impl Into<ListenSocketAddr>,
    ) -> Result<(), BindError> {
        match self.with(|s| (s.bind(local, remote), NoWake)) {
            Ok(()) => Ok(()),
            Err(udp::BindError::InvalidState) => Err(BindError::InvalidState),
            Err(udp::BindError::InUse) => Err(BindError::InUse),
            Err(udp::BindError::NoFreePorts) => Err(BindError::NoFreePorts),
            Err(udp::BindError::Unaddressable) => Err(BindError::Unaddressable),
        }
    }

    /// Bind the socket to an interface, or unbind it with `None`.
    ///
    /// A socket bound to an interface only sends and receives packets on it:
    /// - Destinations must be on-link on that iface, or have a route through it.
    /// - Broadcast and multicast destinations go out on that iface only
    /// - Local addresses will be picked from that iface only.
    ///
    /// The socket must be closed. The binding is kept across
    /// [`close`](Self::close), so a socket stays bound to its interface when
    /// it is bound again.
    ///
    /// Two sockets with otherwise identical tuples may coexist if they are
    /// bound to different interfaces. On ingress, a socket bound to the
    /// arrival interface wins over an unbound one with an equal tuple.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is open.
    #[cfg(feature = "iface-bind")]
    pub fn bind_to_iface(&mut self, iface: Option<IfaceHandle>) -> Result<(), BindError> {
        match self.with(|s| (s.bind_to_iface(iface), NoWake)) {
            Ok(()) => Ok(()),
            Err(udp::BindError::InvalidState) => Err(BindError::InvalidState),
            Err(_) => unreachable!(),
        }
    }

    /// Return the interface the socket is bound to, or `None`.
    ///
    /// See [`bind_to_iface`](Self::bind_to_iface).
    #[cfg(feature = "iface-bind")]
    pub fn bound_iface(&self) -> Option<IfaceHandle> {
        self.with(|s| (s.bound_iface(), NoWake))
    }

    /// Run `f` on the underlying `xarxa` socket, under the stack lock.
    pub(super) fn with<R>(&self, f: impl FnOnce(&mut udp::UdpSocket<'_, 'd>) -> (R, WakeRunner)) -> R {
        self.stack.with(|i| f(&mut i.stack.udp_socket(self.handle)))
    }

    /// Return the bound local address.
    ///
    /// See [`bind`](Self::bind) for details on how UDP socket binding works.
    ///
    /// Returns `ListenSocketAddr::UNSPECIFIED` if not bound.
    pub fn local_addr(&self) -> ListenSocketAddr {
        self.with(|s| (s.local_addr(), NoWake))
    }

    /// Return the bound remote address.
    ///
    /// See [`bind`](Self::bind) for details on how UDP socket binding works.
    ///
    /// Returns `ListenSocketAddr::UNSPECIFIED` if not bound.
    pub fn remote_addr(&self) -> ListenSocketAddr {
        self.with(|s| (s.remote_addr(), NoWake))
    }

    /// Check whether the socket is open (bound to a port).
    pub fn is_open(&self) -> bool {
        self.with(|s| (s.is_open(), NoWake))
    }

    /// Close the socket, unbinding it and dropping any queued packets.
    pub fn close(&mut self) {
        self.with(|s| {
            let freed = s.can_recv();
            s.close();
            ((), wake_if(freed))
        })
    }

    /// Check whether the RX queue is not empty.
    pub fn can_recv(&self) -> bool {
        self.with(|s| (s.can_recv(), NoWake))
    }

    /// Return the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    ///
    /// See also the [set_hop_limit](#method.set_hop_limit) method.
    pub fn hop_limit(&self) -> Option<u8> {
        self.with(|s| (s.hop_limit(), NoWake))
    }

    /// Set the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    ///
    /// A socket without an explicitly set hop limit value uses the default [IANA
    /// recommended] value (64).
    ///
    /// # Errors
    /// - `InvalidHopLimit`: if the hop limit is `Some(0)`. A host must not send a
    ///   packet with a hop limit of zero ([RFC 1122 § 3.2.1.7]). The socket is
    ///   left unchanged.
    ///
    /// [IANA recommended]: https://www.iana.org/assignments/ip-parameters/ip-parameters.xhtml
    /// [RFC 1122 § 3.2.1.7]: https://tools.ietf.org/html/rfc1122#section-3.2.1.7
    pub fn set_hop_limit(&mut self, hop_limit: Option<u8>) -> Result<(), InvalidHopLimit> {
        self.with(|s| (s.set_hop_limit(hop_limit), NoWake))
    }

    /// Allocate an empty UDP payload buffer with headroom for the headers.
    ///
    /// Allocation works before binding. Returns `None` if every packet buffer
    /// is in use. Set the payload length with [`PacketBuf::set_len`] before writing.
    pub fn try_alloc(&self) -> Option<PacketBuf> {
        self.with(|s| (s.alloc(), NoWake))
    }
}