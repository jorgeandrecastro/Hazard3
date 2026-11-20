//! DNS client compatible with the `embedded-nal-async` traits.
//!
//! This exists only for compatibility with crates that use `embedded-nal-async`.
//! Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
//! not using `embedded-nal-async`.

pub(crate) use xarxa::dns::{GetQueryResultError, StartQueryError};
pub use xarxa::wire::{DnsType as DnsQueryType, IpAddr};

/// Errors returned by DnsClient.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The name is empty, or has an empty or too long label.
    InvalidName,
    /// The name is longer than [`DNS_MAX_NAME_SIZE`](crate::config::DNS_MAX_NAME_SIZE) in wire format.
    NameTooLong,
    /// Name lookup failed
    Failed,
}

impl From<GetQueryResultError> for Error {
    fn from(_: GetQueryResultError) -> Self {
        Self::Failed
    }
}

impl From<StartQueryError> for Error {
    fn from(e: StartQueryError) -> Self {
        match e {
            StartQueryError::NoFreeSlot => Self::Failed,
            StartQueryError::InvalidName => Self::InvalidName,
            StartQueryError::NameTooLong => Self::NameTooLong,
        }
    }
}

/// DNS client compatible with the `embedded-nal-async` traits.
///
/// This exists only for compatibility with crates that use `embedded-nal-async`.
/// Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
/// not using `embedded-nal-async`.
#[cfg(feature = "embedded-nal")]
pub struct DnsClient<'d> {
    stack: crate::Stack<'d>,
}

#[cfg(feature = "embedded-nal")]
impl<'d> DnsClient<'d> {
    /// Create a new DNS socket using the provided stack.
    ///
    /// NOTE: If using DHCP, make sure it has reconfigured the stack to ensure the DNS servers are updated.
    pub fn new(stack: crate::Stack<'d>) -> Self {
        Self { stack }
    }

    /// Make a query for a given name and return the corresponding IP addresses.
    pub async fn query(
        &self,
        name: &str,
        qtype: DnsQueryType,
    ) -> Result<heapless::Vec<IpAddr, { xarxa::config::DNS_MAX_RESULT_COUNT }>, Error> {
        self.stack.dns_query(name, qtype).await
    }
}

#[cfg(feature = "embedded-nal")]
impl<'d> embedded_nal_async::Dns for DnsClient<'d> {
    type Error = Error;

    async fn get_host_by_name(
        &self,
        host: &str,
        addr_type: embedded_nal_async::AddrType,
    ) -> Result<core::net::IpAddr, Self::Error> {
        use embedded_nal_async::AddrType;

        let (qtype, secondary_qtype) = match addr_type {
            AddrType::IPv4 => (DnsQueryType::A, None),
            AddrType::IPv6 => (DnsQueryType::Aaaa, None),
            AddrType::Either => {
                #[cfg(not(feature = "ipv6"))]
                let v6_first = false;
                #[cfg(feature = "ipv6")]
                let v6_first = self.stack.any_ipv6();
                match v6_first {
                    true => (DnsQueryType::Aaaa, Some(DnsQueryType::A)),
                    false => (DnsQueryType::A, Some(DnsQueryType::Aaaa)),
                }
            }
        };
        let mut addrs = self.query(host, qtype).await?;
        if addrs.is_empty() {
            if let Some(qtype) = secondary_qtype {
                addrs = self.query(host, qtype).await?
            }
        }
        if let Some(first) = addrs.get(0) {
            Ok(match first {
                #[cfg(feature = "ipv4")]
                IpAddr::V4(addr) => core::net::IpAddr::V4(*addr),
                #[cfg(feature = "ipv6")]
                IpAddr::V6(addr) => core::net::IpAddr::V6(*addr),
            })
        } else {
            Err(Error::Failed)
        }
    }

    async fn get_host_by_address(&self, _addr: core::net::IpAddr, _result: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidName => f.write_str("InvalidName"),
            Self::NameTooLong => f.write_str("NameTooLong"),
            Self::Failed => f.write_str("Failed"),
        }
    }
}
impl core::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;
    use xarxa::dns::{GetQueryResultError, StartQueryError};

    struct BufWriter<'a> {
        buf: &'a mut [u8],
        len: usize,
    }

    impl<'a> BufWriter<'a> {
        fn new(buf: &'a mut [u8]) -> Self {
            Self { buf, len: 0 }
        }
    }

    impl Write for BufWriter<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let rem = self.buf.len().saturating_sub(self.len);
            let n = bytes.len().min(rem);
            self.buf[self.len..self.len + n].copy_from_slice(&bytes[..n]);
            self.len += n;
            if n < bytes.len() { Err(core::fmt::Error) } else { Ok(()) }
        }
    }

    fn display_to_buf<'a>(err: Error, buf: &'a mut [u8]) -> &'a str {
        let len = {
            let mut w = BufWriter::new(buf);
            let _ = write!(w, "{}", err);
            w.len
        };

        core::str::from_utf8(&buf[..len]).unwrap_or("")
    }

    // ---------- Error conversions ----------

    #[test]
    fn from_get_query_result_error_failed() {
        let err: Error = GetQueryResultError::Failed.into();
        assert_eq!(err, Error::Failed);
    }

    #[test]
    fn from_get_query_result_error_pending() {
        // Pending est normalement géré en interne, mais on vérifie le From
        let err: Error = GetQueryResultError::Pending.into();
        assert_eq!(err, Error::Failed);
    }

    #[test]
    fn from_start_query_error_no_free_slot() {
        let err: Error = StartQueryError::NoFreeSlot.into();
        assert_eq!(err, Error::Failed);
    }

    #[test]
    fn from_start_query_error_invalid_name() {
        let err: Error = StartQueryError::InvalidName.into();
        assert_eq!(err, Error::InvalidName);
    }

    #[test]
    fn from_start_query_error_name_too_long() {
        let err: Error = StartQueryError::NameTooLong.into();
        assert_eq!(err, Error::NameTooLong);
    }

    // ---------- Display ----------

    #[test]
    fn error_display() {
        let mut buf = [0u8; 32];
        assert_eq!(display_to_buf(Error::InvalidName, &mut buf), "InvalidName");

        let mut buf = [0u8; 32];
        assert_eq!(display_to_buf(Error::NameTooLong, &mut buf), "NameTooLong");

        let mut buf = [0u8; 32];
        assert_eq!(display_to_buf(Error::Failed, &mut buf), "Failed");
    }

    #[test]
    fn error_is_core_error() {
        let _: &dyn core::error::Error = &Error::Failed;
    }
}
