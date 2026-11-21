//! Error types of the UDP socket.

use xarxa::udp;

#[cfg(feature = "icmp-errors")]
use crate::wire::SocketAddr;
use crate::TryError;

/// Error returned by [`UdpSocket::bind`](super::UdpSocket::bind).
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BindError {
    /// The socket is already bound.
    InvalidState,
    /// Another UDP socket holds an identical 4-tuple.
    InUse,
    /// No free port in the ephemeral range (only possible with tens of thousands
    /// of bound sockets).
    NoFreePorts,
    /// The local and remote addresses belong to different address families, or no
    /// local address is available for the given remote.
    Unaddressable,
}

/// Error returned by [`UdpSocket::send_to`](super::UdpSocket::send_to).
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SendError {
    /// The socket is not bound.
    InvalidState,
    /// The destination address or port is unspecified, or no matching source
    /// address is available.
    Unaddressable,
    /// The payload does not fit in a packet buffer.
    BufferFull,
}

/// Error returned by [`UdpSocket::recv_from`](super::UdpSocket::recv_from).
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RecvError {
    /// The socket is not bound.
    InvalidState,
    /// The provided slice is smaller than the payload. (The packet is dropped.)
    Truncated,
    /// An ICMP error message quoting a packet this socket sent has arrived
    /// (reported once, taking it clears it).
    #[cfg(feature = "icmp-errors")]
    IcmpError {
        /// The kind of error.
        error: crate::error::IcmpError,
        /// The remote address the erring packet was sent to.
        remote: SocketAddr,
    },
}

impl From<udp::SendError> for TryError<SendError> {
    fn from(err: udp::SendError) -> TryError<SendError> {
        match err {
            udp::SendError::DeviceBusy | udp::SendError::NoBuffer => TryError::WouldBlock,
            udp::SendError::BufferFull => TryError::Other(SendError::BufferFull),
            udp::SendError::InvalidState => TryError::Other(SendError::InvalidState),
            udp::SendError::Unaddressable => TryError::Other(SendError::Unaddressable),
        }
    }
}

impl core::fmt::Display for SendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => f.write_str("InvalidState"),
            Self::Unaddressable => f.write_str("Unaddressable"),
            Self::BufferFull => f.write_str("BufferFull"),
        }
    }
}
impl core::error::Error for SendError {}

impl core::fmt::Display for RecvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => f.write_str("InvalidState"),
            Self::Truncated => f.write_str("Truncated"),
            #[cfg(feature = "icmp-errors")]
            Self::IcmpError { .. } => f.write_str("IcmpError"),
        }
    }
}
impl core::error::Error for RecvError {}

impl core::fmt::Display for BindError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => f.write_str("InvalidState"),
            Self::InUse => f.write_str("InUse"),
            Self::NoFreePorts => f.write_str("NoFreePorts"),
            Self::Unaddressable => f.write_str("Unaddressable"),
        }
    }
}
impl core::error::Error for BindError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_variant_names() {
        extern crate std;
        use std::string::ToString;

        assert_eq!(BindError::InUse.to_string(), "InUse");
        assert_eq!(SendError::BufferFull.to_string(), "BufferFull");
        assert_eq!(RecvError::Truncated.to_string(), "Truncated");
    }

    #[test]
    fn send_error_maps_to_try_error() {
        assert!(matches!(
            TryError::from(udp::SendError::DeviceBusy),
            TryError::WouldBlock
        ));
        assert!(matches!(
            TryError::from(udp::SendError::NoBuffer),
            TryError::WouldBlock
        ));
        assert!(matches!(
            TryError::from(udp::SendError::BufferFull),
            TryError::Other(SendError::BufferFull)
        ));
        assert!(matches!(
            TryError::from(udp::SendError::InvalidState),
            TryError::Other(SendError::InvalidState)
        ));
        assert!(matches!(
            TryError::from(udp::SendError::Unaddressable),
            TryError::Other(SendError::Unaddressable)
        ));
    }
}