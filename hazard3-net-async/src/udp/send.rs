//! Sending: `send_to*`, owned-packet sends and the shared send helpers.

use core::future::poll_fn;
use core::task::{Context, Poll};

use hazard3_net::driver::PacketBuf;
use hazard3_net::udp;

use super::{SendError, UdpMetadata, UdpSocket};
use crate::{NoWake, TryError, Wake, wake_if_ok};

impl<'d> UdpSocket<'d> {
    /// Make one send attempt with `f`, and arrange to be polled again if it has to
    /// be retried.
    ///
    /// A datagram that went out wakes the runner: it may be parked on a neighbor
    /// resolution, or have left fragments behind. A refused one changes nothing the
    /// runner acts on, so it doesn't.
    fn poll_send<R>(
        &self,
        cx: &mut Context<'_>,
        f: impl FnOnce(&mut udp::UdpSocket<'_, 'd>) -> Result<R, udp::SendError>,
    ) -> Poll<Result<R, SendError>> {
        self.with(|s| match f(s) {
            Ok(r) => (Poll::Ready(Ok(r)), Wake),
            // xarxa wakes us when the device has room.
            Err(udp::SendError::DeviceBusy) => {
                s.register_send_waker(cx.waker());
                (Poll::Pending, NoWake)
            }
            // Nothing signals a freed buffer. Yield, and try again.
            Err(udp::SendError::NoBuffer) => {
                cx.waker().wake_by_ref();
                (Poll::Pending, NoWake)
            }
            Err(udp::SendError::BufferFull) => (Poll::Ready(Err(SendError::BufferFull)), NoWake),
            Err(udp::SendError::InvalidState) => (Poll::Ready(Err(SendError::InvalidState)), NoWake),
            Err(udp::SendError::Unaddressable) => (Poll::Ready(Err(SendError::Unaddressable)), NoWake),
        })
    }

    /// Make one send attempt with `f`, without waiting. Like
    /// [`poll_send`](Self::poll_send), only a datagram that went out wakes the runner.
    fn try_send_inner<R>(
        &self,
        f: impl FnOnce(&mut udp::UdpSocket<'_, 'd>) -> Result<R, udp::SendError>,
    ) -> Result<R, TryError<SendError>> {
        self.with(|s| wake_if_ok(f(s).map_err(Into::into)))
    }

    /// Send an owned UDP payload, adding the UDP, IP, and link headers.
    ///
    /// The buffer contains only the payload, without a UDP header. Use
    /// [`try_alloc`](Self::try_alloc) to allocate a buffer with suitable headroom.
    /// `remote` selects addresses and replaces the buffer's packet metadata,
    /// as in [`send_to_with`](Self::send_to_with).
    ///
    /// Errors also return the buffer unchanged.
    pub async fn send_packet_to(
        &self,
        buf: PacketBuf,
        remote: impl Into<UdpMetadata>,
    ) -> Result<(), (SendError, PacketBuf)> {
        let remote = remote.into();
        // Each pending attempt restores the packet; success transfers it once.
        let mut buf = Some(buf);
        poll_fn(|cx| {
            self.poll_send(cx, |s| {
                s.send_packet(unwrap!(buf.take()), remote).map_err(|(err, packet)| {
                    buf = Some(packet);
                    err
                })
            })
        })
        .await
        .map_err(|err| (err, unwrap!(buf)))
    }

    /// Try to send an owned packet without waiting or allocating a replacement
    /// payload buffer.
    ///
    /// See [`send_packet_to`](Self::send_packet_to) for buffer layout and ownership.
    /// Every error returns the buffer unchanged. `WouldBlock` means transmission
    /// is temporarily blocked; other errors match [`try_send_to`](Self::try_send_to).
    pub fn try_send_packet_to(
        &self,
        buf: PacketBuf,
        remote: impl Into<UdpMetadata>,
    ) -> Result<(), (TryError<SendError>, PacketBuf)> {
        let remote = remote.into();
        self.with(|s| wake_if_ok(s.send_packet(buf, remote).map_err(|(err, buf)| (err.into(), buf))))
    }

    /// Send a datagram to the given remote address, copying the payload from a slice.
    ///
    /// This method will wait until the datagram has been sent.
    ///
    /// See [send_to_with](#method.send_to_with).
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Unaddressable`: if the destination address or port is still
    ///   unspecified after defaulting, the destination's address family does not
    ///   match the source address, no source address is available, or the source
    ///   address is not assigned to any interface.
    /// - `BufferFull`: if the payload cannot fit in a packet buffer.
    pub async fn send_to(&self, buf: &[u8], remote: impl Into<UdpMetadata>) -> Result<(), SendError> {
        let remote: UdpMetadata = remote.into();
        poll_fn(move |cx| self.poll_send_to(buf, remote, cx)).await
    }

    /// Send a datagram to the given remote address, copying the payload from a slice.
    ///
    /// This method will not wait for a packet buffer or device room to become free.
    ///
    /// See [send_to_with](#method.send_to_with).
    ///
    /// # Errors
    /// - `WouldBlock`: if every packet buffer is in use, or the interface the
    ///   datagram would go out of has no room for it right now.
    /// - `Other(InvalidState)`: if the socket is not bound.
    /// - `Other(Unaddressable)`: if the destination address or port is still
    ///   unspecified after defaulting, the destination's address family does not
    ///   match the source address, no source address is available, or the source
    ///   address is not assigned to any interface.
    /// - `Other(BufferFull)`: if the payload cannot fit in a packet buffer.
    pub fn try_send_to(&self, buf: &[u8], remote: impl Into<UdpMetadata>) -> Result<(), TryError<SendError>> {
        let remote: UdpMetadata = remote.into();
        self.try_send_inner(|s| s.send_slice(buf, remote))
    }

    /// Send a datagram to the given remote address, copying the payload from a slice.
    ///
    /// When the datagram has been sent, this method will return `Poll::Ready(Ok())`.
    ///
    /// When the datagram cannot be sent right now, this method will return `Poll::Pending`
    /// and register the current task to be notified when it can.
    ///
    /// See [send_to_with](#method.send_to_with).
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Unaddressable`: if the destination address or port is still
    ///   unspecified after defaulting, the destination's address family does not
    ///   match the source address, no source address is available, or the source
    ///   address is not assigned to any interface.
    /// - `BufferFull`: if the payload cannot fit in a packet buffer.
    pub fn poll_send_to(
        &self,
        buf: &[u8],
        remote: impl Into<UdpMetadata>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), SendError>> {
        let remote: UdpMetadata = remote.into();
        self.poll_send(cx, |s| s.send_slice(buf, remote))
    }

    /// Send a datagram, building the payload in place.
    ///
    /// The destination is `remote.remote_addr`, with unspecified parts defaulted from
    /// the socket's bound remote address. On a connected socket, sending to
    /// `SocketAddr::UNSPECIFIED` sends to the connected remote. An explicitly
    /// specified destination is honored even on a connected socket.
    ///
    /// The closure gets a `max_size`-byte slice inside a freshly allocated packet
    /// buffer, and returns how many bytes it wrote, along with a value that this
    /// method returns. The datagram is then sent immediately. If the destination's
    /// neighbor is unresolved, the packet is queued inside the stack and sent when
    /// resolution completes. This still counts as a successful send.
    ///
    /// This method will wait until a packet buffer is available before passing
    /// it to the closure.
    ///
    /// `remote.meta` is attached to the packet and handed to the driver with it: an id
    /// to tag the packet with, or a request to timestamp its transmission (see
    /// [`Stack::poll_tx_timestamp`](crate::Stack::poll_tx_timestamp)).
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Unaddressable`: if the destination address or port is still
    ///   unspecified after defaulting, the destination's address family does not
    ///   match the source address, no source address is available, or the source
    ///   address is not assigned to any interface.
    /// - `BufferFull`: if the payload cannot fit in a packet buffer.
    pub async fn send_to_with<R>(
        &mut self,
        max_size: usize,
        remote: impl Into<UdpMetadata>,
        f: impl FnOnce(&mut [u8]) -> (usize, R),
    ) -> Result<R, SendError> {
        let remote = remote.into();
        let mut f = Some(f);
        poll_fn(move |cx| {
            self.poll_send(cx, |s| {
                let mut ret = None;
                s.send_with(max_size, remote, |buf| {
                    let (size, r) = unwrap!(f.take())(buf);
                    ret = Some(r);
                    size
                })
                .map(|()| unwrap!(ret))
            })
        })
        .await
    }

    /// Send a datagram, building the payload in place.
    ///
    /// The destination is `remote.remote_addr`, with unspecified parts defaulted from
    /// the socket's bound remote address. On a connected socket, sending to
    /// `SocketAddr::UNSPECIFIED` sends to the connected remote. An explicitly
    /// specified destination is honored even on a connected socket.
    ///
    /// The closure gets a `size`-byte slice inside a freshly allocated packet
    /// buffer, fills it, and returns a value that this method returns. The datagram
    /// is then sent immediately. If the destination's neighbor is unresolved, the
    /// packet is queued inside the stack and sent when resolution completes. This
    /// still counts as a successful send.
    ///
    /// This method will not wait for a packet buffer to become free.
    ///
    /// `remote.meta` is attached to the packet and handed to the driver with it: an id
    /// to tag the packet with, or a request to timestamp its transmission (see
    /// [`Stack::poll_tx_timestamp`](crate::Stack::poll_tx_timestamp)).
    ///
    /// # Errors
    /// - `WouldBlock`: if every packet buffer is in use, or the interface the
    ///   datagram would go out of has no room for it right now.
    /// - `Other(InvalidState)`: if the socket is not bound.
    /// - `Other(Unaddressable)`: if the destination address or port is still
    ///   unspecified after defaulting, the destination's address family does not
    ///   match the source address, no source address is available, or the source
    ///   address is not assigned to any interface.
    /// - `Other(BufferFull)`: if the payload cannot fit in a packet buffer.
    pub fn try_send_to_with<R>(
        &mut self,
        size: usize,
        remote: impl Into<UdpMetadata>,
        f: impl FnOnce(&mut [u8]) -> R,
    ) -> Result<R, TryError<SendError>> {
        let remote: UdpMetadata = remote.into();
        self.try_send_inner(|s| {
            let mut ret = None;
            s.send_with(size, remote, |buf| {
                ret = Some(f(buf));
                size
            })
            .map(|()| unwrap!(ret))
        })
    }
}

// Keep `SocketAddr` in scope for the `UdpMetadata: From<SocketAddr>` docs links.
#[allow(unused_imports)]
use crate::wire::SocketAddr as _SocketAddr;