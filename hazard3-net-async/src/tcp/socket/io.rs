// SPDX-License-Identifier: MIT OR Apache-2.0

//! Data transfer for TCP sockets: `read`/`write`/`peek`/`flush`, the
//! [`TcpReader`] / [`TcpWriter`] halves, and the `embedded-io` trait impls.

use core::future::{Future, poll_fn};
use core::marker::PhantomData;

use super::{TcpReader, TcpSocket, TcpWriter};
use crate::tcp::Error;
use crate::{NoWake, TryError};

impl<'a, 'd> TcpReader<'a, 'd> {
    /// Wait until the socket becomes readable.
    ///
    /// A socket becomes readable when there is some pending data in the receive buffer, or
    /// when the receive half of the full-duplex connection is closed
    /// (see [`may_recv()`](TcpSocket::may_recv)).
    ///
    /// This is the equivalent of [read](#method.read), without buffering any data.
    pub fn wait_read_ready(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(move |cx| self.io.poll_read_ready(cx))
    }

    /// Read data from the socket.
    ///
    /// Returns how many bytes were read, or an error. If no data is available, it waits
    /// until there is at least one byte available.
    ///
    /// # Note
    /// A return value of Ok(0) means that we have read all data and the remote
    /// side has closed our receive half of the socket. The remote can no longer
    /// send bytes.
    ///
    /// The send half of the socket is still open. If you want to reconnect using
    /// the socket you split this reader off the send half needs to be closed using
    /// [`abort()`](TcpSocket::abort).
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.read(buf).await
    }

    /// Read data from the socket.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, TryError<Error>> {
        self.io.try_read(buf)
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn read_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, Error> {
        self.io.read_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_read_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, TryError<Error>> {
        self.io.try_read_with(f)
    }

    /// Peek at received data without removing it from the receive buffer, copying
    /// it into `buf`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    ///
    /// Like [`read`](Self::read), a return value of Ok(0) means that all data has
    /// been read and the remote side has closed our receive half of the socket.
    pub async fn peek(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.peek(buf).await
    }

    /// Peek at received data without removing it from the receive buffer, copying
    /// it into `buf`.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_peek(&mut self, buf: &mut [u8]) -> Result<usize, TryError<Error>> {
        self.io.try_peek(buf)
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// without removing them from it.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn peek_with<R>(&mut self, f: impl FnOnce(&[u8]) -> R) -> Result<R, Error> {
        self.io.peek_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// without removing them from it.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_peek_with<R>(&mut self, f: impl FnOnce(&[u8]) -> R) -> Result<R, TryError<Error>> {
        self.io.try_peek_with(f)
    }

    /// Return the maximum number of bytes inside the recv buffer.
    pub fn recv_capacity(&self) -> usize {
        self.io.recv_capacity()
    }

    /// Return the amount of octets queued in the receive buffer. This value can be larger than
    /// the slice read by the next `recv` or `peek` call because it includes all queued octets,
    /// and not only the octets that may be returned as a contiguous slice.
    ///
    /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    pub fn recv_queue(&self) -> usize {
        self.io.recv_queue()
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

impl<'a, 'd> TcpWriter<'a, 'd> {
    /// Wait until the socket becomes writable.
    ///
    /// A socket becomes writable when the transmit buffer is not full, or when the transmit
    /// half of the full-duplex connection is closed (see [`may_send()`](TcpSocket::may_send)).
    ///
    /// This is the equivalent of [write](#method.write), without sending any data.
    pub fn wait_write_ready(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(move |cx| self.io.poll_write_ready(cx))
    }

    /// Write data to the socket.
    ///
    /// Returns how many bytes were written, or an error. If the socket is not ready to
    /// accept data, it waits until it is.
    pub fn write<'s>(&'s mut self, buf: &'s [u8]) -> impl Future<Output = Result<usize, Error>> + 's {
        self.io.write(buf)
    }

    /// Write data to the socket.
    ///
    /// This method will not wait for the buffer to become free.
    ///
    /// If the socket's send buffer is full, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_write(&mut self, buf: &[u8]) -> Result<usize, TryError<Error>> {
        self.io.try_write(buf)
    }

    /// Flushes the written data to the socket.
    ///
    /// This waits until all data has been sent, and ACKed by the remote host. For a connection
    /// closed with [`abort()`](TcpSocket::abort) it will wait for the TCP RST packet to be sent.
    pub fn flush(&mut self) -> impl Future<Output = Result<(), Error>> + '_ {
        self.io.flush()
    }

    /// Try to flush the socket.
    ///
    /// This method will check if the socket is flushed, and if not, return `Err(TryError::WouldBlock)`.
    pub fn try_flush(&mut self) -> Result<(), TryError<Error>> {
        self.io.try_flush()
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// If the socket is not ready to accept data, it waits until it is.
    pub async fn write_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, Error> {
        self.io.write_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// This method will not wait for the buffer to become free.
    ///
    /// If the socket's send buffer is full, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_write_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, TryError<Error>> {
        self.io.try_write_with(f)
    }

    /// Return the maximum number of bytes inside the transmit buffer.
    pub fn send_capacity(&self) -> usize {
        self.io.send_capacity()
    }

    /// Return the amount of octets queued in the transmit buffer.
    ///
    /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    pub fn send_queue(&self) -> usize {
        self.io.send_queue()
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
    /// This function returns true if it's possible for the corresponding [`TcpReader`]
    /// to receive data from the remote peer.
    /// It will return true while there is data in the receive buffer, and if there isn't,
    /// as long as the remote peer has not closed the connection.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED`,
    /// `FIN-WAIT-1`, or `FIN-WAIT-2` state, or have data in the receive buffer instead.
    pub fn may_recv(&self) -> bool {
        self.io.with(|s| (s.may_recv(), NoWake))
    }
}

impl<'a, 'd> TcpSocket<'a, 'd> {
    /// Return the maximum number of bytes inside the recv buffer.
    pub fn recv_capacity(&self) -> usize {
        self.io.recv_capacity()
    }

    /// Return the maximum number of bytes inside the transmit buffer.
    pub fn send_capacity(&self) -> usize {
        self.io.send_capacity()
    }

    /// Return the amount of octets queued in the transmit buffer.
    ///
    /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    pub fn send_queue(&self) -> usize {
        self.io.send_queue()
    }

    /// Return the amount of octets queued in the receive buffer. This value can be larger than
    /// the slice read by the next `recv` or `peek` call because it includes all queued octets,
    /// and not only the octets that may be returned as a contiguous slice.
    ///
    /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    pub fn recv_queue(&self) -> usize {
        self.io.recv_queue()
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// If the socket is not ready to accept data, it waits until it is.
    pub async fn write_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, Error> {
        self.io.write_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// This method will not wait for the buffer to become free.
    ///
    /// If the socket's send buffer is full, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_write_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, TryError<Error>> {
        self.io.try_write_with(f)
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn read_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, Error> {
        self.io.read_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_read_with<R>(&mut self, f: impl FnOnce(&mut [u8]) -> (usize, R)) -> Result<R, TryError<Error>> {
        self.io.try_read_with(f)
    }

    /// Peek at received data without removing it from the receive buffer, copying
    /// it into `buf`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    ///
    /// Like [`read`](Self::read), a return value of Ok(0) means that all data has
    /// been read and the remote side has closed our receive half of the socket.
    pub async fn peek(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.peek(buf).await
    }

    /// Peek at received data without removing it from the receive buffer, copying
    /// it into `buf`.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_peek(&mut self, buf: &mut [u8]) -> Result<usize, TryError<Error>> {
        self.io.try_peek(buf)
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// without removing them from it.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn peek_with<R>(&mut self, f: impl FnOnce(&[u8]) -> R) -> Result<R, Error> {
        self.io.peek_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// without removing them from it.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_peek_with<R>(&mut self, f: impl FnOnce(&[u8]) -> R) -> Result<R, TryError<Error>> {
        self.io.try_peek_with(f)
    }

    /// Split the socket into reader and a writer halves.
    pub fn split(&mut self) -> (TcpReader<'_, 'd>, TcpWriter<'_, 'd>) {
        (
            TcpReader {
                io: self.io,
                bufs: PhantomData,
            },
            TcpWriter {
                io: self.io,
                bufs: PhantomData,
            },
        )
    }

    /// Wait until the socket becomes readable.
    ///
    /// A socket becomes readable when there is some pending data in the receive buffer, or
    /// when the receive half of the full-duplex connection is closed
    /// (see [may_recv](#method.may_recv)).
    ///
    /// This is the equivalent of [read](#method.read), without buffering any data.
    pub fn wait_read_ready(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(move |cx| self.io.poll_read_ready(cx))
    }

    /// Read data from the socket.
    ///
    /// Returns how many bytes were read, or an error. If no data is available, it waits
    /// until there is at least one byte available.
    ///
    /// A return value of Ok(0) means that the socket was closed and is longer
    /// able to receive any data.
    pub fn read<'s>(&'s mut self, buf: &'s mut [u8]) -> impl Future<Output = Result<usize, Error>> + 's {
        self.io.read(buf)
    }

    /// Read data from the socket.
    ///
    /// This method will not wait for data to be received.
    ///
    /// If no data is available, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, TryError<Error>> {
        self.io.try_read(buf)
    }

    /// Wait until the socket becomes writable.
    ///
    /// A socket becomes writable when the transmit buffer is not full, or when the transmit
    /// half of the full-duplex connection is closed (see [may_send](#method.may_send)).
    ///
    /// This is the equivalent of [write](#method.write), without sending any data.
    pub fn wait_write_ready(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(move |cx| self.io.poll_write_ready(cx))
    }

    /// Write data to the socket.
    ///
    /// Returns how many bytes were written, or an error. If the socket is not ready to
    /// accept data, it waits until it is.
    pub fn write<'s>(&'s mut self, buf: &'s [u8]) -> impl Future<Output = Result<usize, Error>> + 's {
        self.io.write(buf)
    }

    /// Write data to the socket.
    ///
    /// This method will not wait for the buffer to become free.
    ///
    /// If the socket's send buffer is full, this method will return `Err(TryError::WouldBlock)`.
    pub fn try_write(&mut self, buf: &[u8]) -> Result<usize, TryError<Error>> {
        self.io.try_write(buf)
    }

    /// Flushes the written data to the socket.
    ///
    /// This waits until all data has been sent, and ACKed by the remote host. For a connection
    /// closed with [`abort()`](TcpSocket::abort) it will wait for the TCP RST packet to be sent.
    pub fn flush(&mut self) -> impl Future<Output = Result<(), Error>> + '_ {
        self.io.flush()
    }

    /// Try to flush the socket.
    ///
    /// This method will check if the socket is flushed, and if not, return `Err(TryError::WouldBlock)`.
    pub fn try_flush(&mut self) -> Result<(), TryError<Error>> {
        self.io.try_flush()
    }
}

mod embedded_io_impls {
    use super::*;

    impl embedded_io_async::ErrorType for TcpSocket<'_, '_> {
        type Error = Error;
    }

    impl embedded_io_async::Read for TcpSocket<'_, '_> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.io.read(buf).await
        }
    }

    impl embedded_io_async::ReadReady for TcpSocket<'_, '_> {
        fn read_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| (s.can_recv() || !s.may_recv(), NoWake)))
        }
    }

    impl embedded_io_async::Write for TcpSocket<'_, '_> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.io.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.io.flush().await
        }
    }

    impl embedded_io_async::WriteReady for TcpSocket<'_, '_> {
        fn write_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| (s.can_send() || !s.may_send(), NoWake)))
        }
    }

    impl embedded_io_async::ErrorType for TcpReader<'_, '_> {
        type Error = Error;
    }

    impl embedded_io_async::Read for TcpReader<'_, '_> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.io.read(buf).await
        }
    }

    impl embedded_io_async::ReadReady for TcpReader<'_, '_> {
        fn read_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| (s.can_recv() || !s.may_recv(), NoWake)))
        }
    }

    impl embedded_io_async::ErrorType for TcpWriter<'_, '_> {
        type Error = Error;
    }

    impl embedded_io_async::Write for TcpWriter<'_, '_> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.io.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.io.flush().await
        }
    }

    impl embedded_io_async::WriteReady for TcpWriter<'_, '_> {
        fn write_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| (s.can_send() || !s.may_send(), NoWake)))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Compile-time checks of the data-transfer API and `embedded-io` impls.
    //! Runtime tests need a `Stack` with a mock driver.

    use embedded_io_async::{ErrorType, Read, ReadReady, Write, WriteReady};

    use super::*;

    fn assert_read<T: Read + ReadReady>() {}
    fn assert_write<T: Write + WriteReady>() {}
    fn assert_error_type<T: ErrorType<Error = Error>>() {}
    fn assert_output<T, F: Future<Output = T>>(_: F) {}

    #[test]
    fn socket_implements_embedded_io() {
        assert_error_type::<TcpSocket<'static, 'static>>();
        assert_read::<TcpSocket<'static, 'static>>();
        assert_write::<TcpSocket<'static, 'static>>();
    }

    #[test]
    fn reader_implements_embedded_io_read() {
        assert_error_type::<TcpReader<'static, 'static>>();
        assert_read::<TcpReader<'static, 'static>>();
    }

    #[test]
    fn writer_implements_embedded_io_write() {
        assert_error_type::<TcpWriter<'static, 'static>>();
        assert_write::<TcpWriter<'static, 'static>>();
    }

    #[allow(dead_code)]
    fn socket_io_api(s: &mut TcpSocket<'_, '_>, buf: &mut [u8], wbuf: &[u8]) {
        assert_output::<Result<usize, Error>, _>(s.read(buf));
        assert_output::<Result<usize, Error>, _>(s.write(wbuf));
        assert_output::<Result<usize, Error>, _>(s.peek(buf));
        assert_output::<Result<(), Error>, _>(s.flush());
        assert_output::<(), _>(s.wait_read_ready());
        assert_output::<(), _>(s.wait_write_ready());

        let _: Result<usize, TryError<Error>> = s.try_read(buf);
        let _: Result<usize, TryError<Error>> = s.try_write(wbuf);
        let _: Result<usize, TryError<Error>> = s.try_peek(buf);
        let _: Result<(), TryError<Error>> = s.try_flush();

        let _: usize = s.recv_capacity();
        let _: usize = s.send_capacity();
        let _: usize = s.recv_queue();
        let _: usize = s.send_queue();
    }

    #[allow(dead_code)]
    fn socket_closure_api(s: &mut TcpSocket<'_, '_>) {
        assert_output::<Result<u8, Error>, _>(s.read_with(|b| (0, b.len() as u8)));
        assert_output::<Result<u8, Error>, _>(s.write_with(|b| (0, b.len() as u8)));
        assert_output::<Result<u8, Error>, _>(s.peek_with(|b| b.len() as u8));
        let _: Result<u8, TryError<Error>> = s.try_read_with(|b| (0, b.len() as u8));
        let _: Result<u8, TryError<Error>> = s.try_write_with(|b| (0, b.len() as u8));
        let _: Result<u8, TryError<Error>> = s.try_peek_with(|b| b.len() as u8);
    }

    #[allow(dead_code)]
    fn split_api(s: &mut TcpSocket<'_, '_>, buf: &mut [u8], wbuf: &[u8]) {
        let (mut r, mut w) = s.split();
        assert_output::<Result<usize, Error>, _>(r.read(buf));
        assert_output::<Result<usize, Error>, _>(w.write(wbuf));
        assert_output::<Result<(), Error>, _>(w.flush());
        let _: bool = r.may_recv();
        let _: bool = r.can_recv();
        let _: bool = w.may_send();
        let _: bool = w.can_send();
    }
}