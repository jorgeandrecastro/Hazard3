//! UDP sockets.
//!
//! How to use:
//!
//! - Create a UDP socket with [`UdpSocket::new`]
//! - [`bind`](UdpSocket::bind) it to a local address and optionally also a remote address.
//! - Send and receive packets.

mod error;
mod recv;
mod send;
mod socket;

pub use xarxa::driver::PacketMeta;
#[cfg(feature = "iface-bind")]
pub use xarxa::iface::IfaceHandle;
use xarxa::udp::UdpHandle;
pub use xarxa::udp::{RecvPacket, UdpMetadata};

pub use self::error::{BindError, RecvError, SendError};
use crate::{Stack, wake_if};

/// An UDP socket.
pub struct UdpSocket<'d> {
    stack: Stack<'d>,
    handle: UdpHandle,
}

impl Drop for UdpSocket<'_> {
    fn drop(&mut self) {
        self.stack.with(|i| {
            let freed = i.stack.udp_socket(self.handle).can_recv();
            i.stack.remove_udp_socket(self.handle);
            ((), wake_if(freed))
        });
    }
}