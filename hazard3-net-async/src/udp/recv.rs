//! Receiving: `recv*`, `peek*` and readiness.

use core::future::{Future, poll_fn};
use core::task::{Context, Poll};

use hazard3_net::udp;

use super::{RecvError, RecvPacket, UdpMetadata, UdpSocket};
use crate::{NoWake, TryError, Wake};

impl<'d> UdpSocket<'d> {
    /// Wait until the socket becomes readable.
    ///
    /// A socket is readable when a packet has been received, or when there are queued packets in
    /// the buffer.
    pub fn wait_recv_ready(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(move |cx| self.poll_recv_ready(cx))
    }

    /// Wait until a datagram can be read.
    ///
    /// When no datagram is readable, this method will return `Poll::Pending` and
    /// register the current task to be notified when a datagram is received.
    ///
    /// When a datagram is received, this method will return `Poll::Ready`.
    pub fn poll_recv_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        self.with(|s| {
            (
                if s.can_recv() {
                    Poll::Ready(())
                } else {
                    // socket buffer is empty wait until at least one byte has arrived
                    s.register_recv_waker(cx.waker());
                    Poll::Pending
                },
                NoWake,
            )
        })
    }

    /// Dequeue a received datagram, copying the payload into the given slice, and
    /// return the number of octets copied along with its metadata.
    ///
    /// This method will wait until a datagram is received.
    ///
    /// See also [recv](#method.recv).
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Truncated`: if `buf` is smaller than the payload. The packet is
    ///   dropped.
    /// - `IcmpError`: with the `icmp-errors` feature, if an ICMP error is
    ///   pending. See [recv](#method.recv).
    pub fn recv_from<'s>(
        &'s self,
        buf: &'s mut [u8],
    ) -> impl Future<Output = Result<(usize, UdpMetadata), RecvError>> + 's {
        poll_fn(|cx| self.poll_recv_from(buf, cx))
    }

    /// Dequeue a received datagram, copying the payload into the given slice, and
    /// return the number of octets copied along with its metadata.
    ///
    /// This method will not wait for a datagram to be received.
    ///
    /// See also [try_recv](#method.try_recv).
    ///
    /// # Errors
    /// - `WouldBlock`: if no datagram is available.
    /// - `Other(InvalidState)`: if the socket is not bound.
    /// - `Other(Truncated)`: if `buf` is smaller than the payload. The packet is
    ///   dropped.
    /// - `Other(IcmpError)`: with the `icmp-errors` feature, if an ICMP error is
    ///   pending. See [recv](#method.recv).
    pub fn try_recv_from(&self, buf: &mut [u8]) -> Result<(usize, UdpMetadata), TryError<RecvError>> {
        self.with(|s| match s.recv_slice(buf) {
            Ok((n, meta)) => (Ok((n, meta)), Wake),
            Err(udp::RecvError::InvalidState) => (Err(TryError::Other(RecvError::InvalidState)), NoWake),
            #[cfg(feature = "icmp-errors")]
            Err(udp::RecvError::IcmpError { error, remote }) => {
                (Err(TryError::Other(RecvError::IcmpError { error, remote })), NoWake)
            }
            Err(udp::RecvError::Truncated) => (Err(TryError::Other(RecvError::Truncated)), Wake),
            Err(udp::RecvError::Exhausted) => (Err(TryError::WouldBlock), NoWake),
        })
    }

    /// Dequeue a received datagram, copying the payload into the given slice, and
    /// return the number of octets copied along with its metadata.
    ///
    /// When no datagram is available, this method will return `Poll::Pending` and
    /// register the current task to be notified when a datagram is received.
    ///
    /// When a datagram is received, this method will return `Poll::Ready` with the
    /// number of bytes received and the remote address.
    ///
    /// See also [recv](#method.recv).
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Truncated`: if `buf` is smaller than the payload. The packet is
    ///   dropped.
    /// - `IcmpError`: with the `icmp-errors` feature, if an ICMP error is
    ///   pending. See [recv](#method.recv).
    pub fn poll_recv_from(
        &self,
        buf: &mut [u8],
        cx: &mut Context<'_>,
    ) -> Poll<Result<(usize, UdpMetadata), RecvError>> {
        self.with(|s| match s.recv_slice(buf) {
            Ok((n, meta)) => (Poll::Ready(Ok((n, meta))), Wake),
            Err(udp::RecvError::InvalidState) => (Poll::Ready(Err(RecvError::InvalidState)), NoWake),
            #[cfg(feature = "icmp-errors")]
            Err(udp::RecvError::IcmpError { error, remote }) => {
                (Poll::Ready(Err(RecvError::IcmpError { error, remote })), NoWake)
            }
            Err(udp::RecvError::Truncated) => (Poll::Ready(Err(RecvError::Truncated)), Wake),
            // No data ready
            Err(udp::RecvError::Exhausted) => {
                s.register_recv_waker(cx.waker());
                (Poll::Pending, NoWake)
            }
        })
    }

    /// Dequeue a received datagram, as an owned packet ([`RecvPacket`]).
    ///
    /// This is zero-copy: the returned value is the buffer the datagram arrived in.
    ///
    /// This method will wait until a datagram is received.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `IcmpError`: with the `icmp-errors` feature, if an ICMP error is
    ///   pending. It is reported before any queued datagram, once, and taking
    ///   it clears it.
    pub async fn recv(&self) -> Result<RecvPacket, RecvError> {
        poll_fn(|cx| {
            self.with(|s| match s.recv() {
                Ok(packet) => (Poll::Ready(Ok(packet)), Wake),
                Err(udp::RecvError::InvalidState) => (Poll::Ready(Err(RecvError::InvalidState)), NoWake),
                #[cfg(feature = "icmp-errors")]
                Err(udp::RecvError::IcmpError { error, remote }) => {
                    (Poll::Ready(Err(RecvError::IcmpError { error, remote })), NoWake)
                }
                Err(udp::RecvError::Truncated) => unreachable!(),
                Err(udp::RecvError::Exhausted) => {
                    s.register_recv_waker(cx.waker());
                    (Poll::Pending, NoWake)
                }
            })
        })
        .await
    }

    /// Dequeue a received datagram, as an owned packet ([`RecvPacket`]).
    ///
    /// This is zero-copy: the returned value is the buffer the datagram arrived in.
    ///
    /// This method will not wait for a datagram to be received.
    ///
    /// # Errors
    /// - `WouldBlock`: if no datagram is available.
    /// - `Other(InvalidState)`: if the socket is not bound.
    /// - `Other(IcmpError)`: with the `icmp-errors` feature, if an ICMP error is
    ///   pending. It is reported before any queued datagram, once, and taking
    ///   it clears it.
    pub fn try_recv(&self) -> Result<RecvPacket, TryError<RecvError>> {
        self.with(|s| match s.recv() {
            Ok(packet) => (Ok(packet), Wake),
            Err(udp::RecvError::InvalidState) => (Err(TryError::Other(RecvError::InvalidState)), NoWake),
            #[cfg(feature = "icmp-errors")]
            Err(udp::RecvError::IcmpError { error, remote }) => {
                (Err(TryError::Other(RecvError::IcmpError { error, remote })), NoWake)
            }
            Err(udp::RecvError::Truncated) => unreachable!(),
            Err(udp::RecvError::Exhausted) => (Err(TryError::WouldBlock), NoWake),
        })
    }

    /// Receive a datagram with a zero-copy function.
    ///
    /// When no datagram is available, this method will return `Poll::Pending` and
    /// register the current task to be notified when a datagram is received.
    ///
    /// When a datagram is received, this method will call the provided function
    /// with a reference to the received bytes and the remote address and return
    /// `Poll::Ready` with the function's returned value.
    pub async fn recv_from_with<R>(&mut self, f: impl FnOnce(&[u8], UdpMetadata) -> R) -> Result<R, RecvError> {
        let packet = self.recv().await?;
        Ok(f(packet.payload(), packet.meta()))
    }

    /// Receive a datagram with a zero-copy function.
    ///
    /// This method will not wait for a datagram to be received.
    ///
    /// If no datagram is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_recv_from_with<R>(&mut self, f: impl FnOnce(&[u8], UdpMetadata) -> R) -> Result<R, TryError<RecvError>> {
        let packet = self.try_recv()?;
        Ok(f(packet.payload(), packet.meta()))
    }

    /// Peek at the next received datagram without dequeueing it, copying the payload
    /// into the given slice.
    ///
    /// This method will wait until a datagram is received.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Truncated`: if `buf` is smaller than the payload. No data is copied
    ///   and the packet stays in the queue.
    pub fn peek_from<'s>(
        &'s self,
        buf: &'s mut [u8],
    ) -> impl Future<Output = Result<(usize, UdpMetadata), RecvError>> + 's {
        poll_fn(|cx| self.poll_peek_from(buf, cx))
    }

    /// Peek at the next received datagram without dequeueing it, copying the payload
    /// into the given slice.
    ///
    /// This method will not wait for a datagram to be received.
    ///
    /// # Errors
    /// - `WouldBlock`: if no datagram is available.
    /// - `Other(InvalidState)`: if the socket is not bound.
    /// - `Other(Truncated)`: if `buf` is smaller than the payload. No data is copied
    ///   and the packet stays in the queue.
    pub fn try_peek_from(&self, buf: &mut [u8]) -> Result<(usize, UdpMetadata), TryError<RecvError>> {
        self.with(|s| {
            (
                match s.peek_slice(buf) {
                    Ok((n, meta)) => Ok((n, meta)),
                    Err(udp::RecvError::InvalidState) => Err(TryError::Other(RecvError::InvalidState)),
                    Err(udp::RecvError::Truncated) => Err(TryError::Other(RecvError::Truncated)),
                    Err(udp::RecvError::Exhausted) => Err(TryError::WouldBlock),
                    #[cfg(feature = "icmp-errors")]
                    Err(udp::RecvError::IcmpError { .. }) => unreachable!(),
                },
                NoWake,
            )
        })
    }

    /// Peek at the next received datagram without dequeueing it, copying the payload
    /// into the given slice.
    ///
    /// When no datagram is available, this method will return `Poll::Pending` and
    /// register the current task to be notified when a datagram is received.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    /// - `Truncated`: if `buf` is smaller than the payload. No data is copied
    ///   and the packet stays in the queue.
    pub fn poll_peek_from(
        &self,
        buf: &mut [u8],
        cx: &mut Context<'_>,
    ) -> Poll<Result<(usize, UdpMetadata), RecvError>> {
        self.with(|s| {
            (
                match s.peek_slice(buf) {
                    Ok((n, meta)) => Poll::Ready(Ok((n, meta))),
                    Err(udp::RecvError::InvalidState) => Poll::Ready(Err(RecvError::InvalidState)),
                    Err(udp::RecvError::Truncated) => Poll::Ready(Err(RecvError::Truncated)),
                    Err(udp::RecvError::Exhausted) => {
                        s.register_recv_waker(cx.waker());
                        Poll::Pending
                    }
                    #[cfg(feature = "icmp-errors")]
                    Err(udp::RecvError::IcmpError { .. }) => unreachable!(),
                },
                NoWake,
            )
        })
    }

    /// Peek at the next received datagram without dequeueing it, calling `f` with
    /// its payload and its metadata.
    ///
    /// This method will wait until a datagram is received.
    ///
    /// # Errors
    /// - `InvalidState`: if the socket is not bound.
    pub async fn peek_from_with<R>(&self, f: impl FnOnce(&[u8], UdpMetadata) -> R) -> Result<R, RecvError> {
        let mut f = Some(f);
        poll_fn(|cx| {
            self.with(|s| {
                (
                    match s.peek() {
                        Ok((payload, meta)) => Poll::Ready(Ok(unwrap!(f.take())(payload, meta))),
                        Err(udp::RecvError::InvalidState) => Poll::Ready(Err(RecvError::InvalidState)),
                        Err(udp::RecvError::Exhausted) => {
                            s.register_recv_waker(cx.waker());
                            Poll::Pending
                        }
                        Err(udp::RecvError::Truncated) => unreachable!(),
                        #[cfg(feature = "icmp-errors")]
                        Err(udp::RecvError::IcmpError { .. }) => unreachable!(),
                    },
                    NoWake,
                )
            })
        })
        .await
    }

    /// Peek at the next received datagram without dequeueing it, calling `f` with
    /// its payload and its metadata.
    ///
    /// This method will not wait for a datagram to be received.
    ///
    /// # Errors
    /// - `WouldBlock`: if no datagram is available.
    /// - `Other(InvalidState)`: if the socket is not bound.
    pub fn try_peek_from_with<R>(&self, f: impl FnOnce(&[u8], UdpMetadata) -> R) -> Result<R, TryError<RecvError>> {
        self.with(|s| {
            (
                match s.peek() {
                    Ok((payload, meta)) => Ok(f(payload, meta)),
                    Err(udp::RecvError::InvalidState) => Err(TryError::Other(RecvError::InvalidState)),
                    Err(udp::RecvError::Exhausted) => Err(TryError::WouldBlock),
                    Err(udp::RecvError::Truncated) => unreachable!(),
                    #[cfg(feature = "icmp-errors")]
                    Err(udp::RecvError::IcmpError { .. }) => unreachable!(),
                },
                NoWake,
            )
        })
    }
}