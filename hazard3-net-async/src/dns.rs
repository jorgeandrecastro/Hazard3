//! DNS client compatible with the `embedded-nal-async` traits.
//!
//! This exists only for compatibility with crates that use `embedded-nal-async`.
//! Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
//! not using `embedded-nal-async`.

pub(crate) use hazard3_net::dns::{GetQueryResultError, StartQueryError};
pub use hazard3_net::wire::{DnsType as DnsQueryType, IpAddr};

/// Errors returned by DnsClient.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The name is empty, or has an empty or too long label.
    InvalidName,
    /// The name is longer than [`DNS_MAX_NAME_SIZE`](crate::config::DNS_MAX_NAME_SIZE) in wire format.
    NameTooLong,
    /// Name lookup failed
    Failed,
    /// The requested operation is not supported (e.g. reverse lookup).
    Unsupported,
}

impl From<GetQueryResultError> for Error {
    fn from(_: GetQueryResultError) -> Self {
        // `Pending` is handled internally by `Stack::dns_query`, so every error
        // that reaches this point is a failed lookup.
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
/// See the [module documentation](self) for when to use it.
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
    ) -> Result<heapless::Vec<IpAddr, { hazard3_net::config::DNS_MAX_RESULT_COUNT }>, Error> {
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

        let mut result = self.stack.dns_query(host, qtype).await;

        // For `AddrType::Either`, fall back to the other record type if the first
        // query returned nothing or failed. Retrying is pointless when the name
        // itself is invalid, since the second query would fail identically.
        if !matches!(&result, Ok(a) if !a.is_empty()) {
            if let Some(qtype) = secondary_qtype {
                if !matches!(result, Err(Error::InvalidName | Error::NameTooLong)) {
                    result = self.stack.dns_query(host, qtype).await;
                }
            }
        }

        match result?.first() {
            #[cfg(feature = "ipv4")]
            Some(IpAddr::V4(addr)) => Ok(core::net::IpAddr::V4(*addr)),
            #[cfg(feature = "ipv6")]
            Some(IpAddr::V6(addr)) => Ok(core::net::IpAddr::V6(*addr)),
            None => Err(Error::Failed),
        }
    }

    async fn get_host_by_address(&self, _addr: core::net::IpAddr, _result: &mut [u8]) -> Result<usize, Self::Error> {
        // Reverse lookups (PTR) are not implemented.
        Err(Error::Unsupported)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidName => f.write_str("InvalidName"),
            Self::NameTooLong => f.write_str("NameTooLong"),
            Self::Failed => f.write_str("Failed"),
            Self::Unsupported => f.write_str("Unsupported"),
        }
    }
}

impl core::error::Error for Error {}

#[cfg(test)]
mod tests {
    use core::fmt::Write;

    use super::*;

    fn display(err: Error) -> heapless::String<32> {
        let mut s = heapless::String::new();
        write!(s, "{}", err).unwrap();
        s
    }

    // ---------- Error conversions ----------

    #[test]
    fn from_get_query_result_error_failed() {
        let err: Error = GetQueryResultError::Failed.into();
        assert_eq!(err, Error::Failed);
    }

    #[test]
    fn from_get_query_result_error_pending() {
        // `Pending` is normally handled internally, but the conversion must still be defined.
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
        assert_eq!(display(Error::InvalidName).as_str(), "InvalidName");
        assert_eq!(display(Error::NameTooLong).as_str(), "NameTooLong");
        assert_eq!(display(Error::Failed).as_str(), "Failed");
        assert_eq!(display(Error::Unsupported).as_str(), "Unsupported");
    }

    #[test]
    fn error_is_core_error() {
        let _: &dyn core::error::Error = &Error::Failed;
    }
}