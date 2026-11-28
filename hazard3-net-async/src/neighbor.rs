//! Neighbor cache.

use hazard3_time::Instant;
use hazard3_net::error::NotUnicast;
use hazard3_net::wire::{HardwareAddress, IpAddr};

use crate::iface::IfaceHandle;
use crate::time::{instant_from_xarxa, instant_to_xarxa};
use crate::{NoWake, Stack, wake_if_ok};

/// An entry in the [`NeighborCache`].
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    /// Interface the neighbor is reachable through.
    pub iface: IfaceHandle,
    /// The neighbor's IP address.
    pub addr: IpAddr,
    /// Whether the hardware address is known yet.
    pub state: NeighborState,
}

impl Neighbor {
    fn from_xarxa(neighbor: hazard3_net::Neighbor) -> Self {
        Self {
            iface: neighbor.iface,
            addr: neighbor.addr,
            state: match neighbor.state {
                hazard3_net::NeighborState::Incomplete => NeighborState::Incomplete,
                hazard3_net::NeighborState::Reachable {
                    hardware_addr,
                    expires_at,
                } => NeighborState::Reachable {
                    hardware_addr,
                    expires_at: instant_from_xarxa(expires_at),
                },
                hazard3_net::NeighborState::Stale { hardware_addr } => NeighborState::Stale { hardware_addr },
            },
        }
    }
}

/// State of a [`Neighbor`] entry.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborState {
    /// Address resolution is in progress. Packets for this neighbor are parked
    /// until it resolves or resolution gives up.
    Incomplete,
    /// The neighbor's hardware address is known.
    Reachable {
        /// The neighbor's hardware address.
        hardware_addr: HardwareAddress,
        /// When the entry expires.
        expires_at: Instant,
    },
    /// The entry expired. The stack no longer sends to this hardware address.
    /// The next packet for the neighbor resolves it again.
    ///
    /// Traffic from the neighbor with the same hardware address makes the entry
    /// reachable again.
    Stale {
        /// The neighbor's hardware address, when it was last known.
        hardware_addr: HardwareAddress,
    },
}

/// The neighbor cache: the stack's map of IP addresses to hardware addresses.
///
/// It holds one entry per neighbor, keyed by the interface it is reachable
/// through plus its IP address. Entries are filled in by ARP and neighbor
/// discovery, and expire after 60 s unless traffic from the neighbor refreshes
/// them.
///
/// Access it with [`Stack::neighbor_cache`].
#[derive(Copy, Clone)]
pub struct NeighborCache<'d> {
    stack: Stack<'d>,
}

impl<'d> NeighborCache<'d> {
    #[inline]
    pub(crate) fn new(stack: Stack<'d>) -> Self {
        Self { stack }
    }

    /// Get the entry for a neighbor.
    ///
    /// An entry that expired is reported as [`NeighborState::Stale`] from the
    /// next poll on, until the stack reuses its slot. Before that poll it is
    /// still `Reachable`, with an `expires_at` that has passed.
    pub fn get(&self, iface: IfaceHandle, addr: IpAddr) -> Option<Neighbor> {
        self.stack
            .with(|i| (i.stack.neighbor_cache().get(iface, addr), NoWake))
            .map(Neighbor::from_xarxa)
    }

    /// Iterate over all entries.
    ///
    /// This locks the stack once per entry, and each step walks the cache from
    /// the start, so a full pass costs O(n²). The lock is released between
    /// entries: if the stack changes the cache meanwhile (an ARP reply, an
    /// expiry, an eviction), the pass can skip an entry or return one twice.
    ///
    /// Prefer [`NeighborCache::for_each`], which makes one pass under one lock.
    pub fn iter(&self) -> impl Iterator<Item = Neighbor> + 'd {
        let stack = self.stack;
        (0..self.len())
            .filter_map(move |n| stack.with(|i| (i.stack.neighbor_cache().iter().nth(n), NoWake)))
            .map(Neighbor::from_xarxa)
    }

    /// Call `f` for every entry, in one pass.
    ///
    /// The stack is locked for the whole call, so `f` sees a consistent view of
    /// the cache, and the pass is O(n). Keep `f` short, and do not call the
    /// stack from it: that would deadlock or panic.
    pub fn for_each(&self, mut f: impl FnMut(Neighbor)) {
        self.stack.with(|i| {
            for n in i.stack.neighbor_cache().iter() {
                f(Neighbor::from_xarxa(n));
            }
            ((), NoWake)
        })
    }

    /// Add or replace an entry, mapping `addr` on `iface` to `hardware_addr`.
    ///
    /// `expires_at` is when the entry stops being used. There are no static
    /// entries. To keep an entry, insert it again before it expires. An
    /// `expires_at` more than ~12 days away is clamped to that.
    ///
    /// The stack changes the entry too:
    /// - Traffic from the neighbor sets it to expire 60 s later.
    /// - ARP or neighbor discovery replaces it if the neighbor answers with a
    ///   different hardware address.
    ///
    /// If the cache is full, another entry is evicted to make room.
    ///
    /// # Errors
    /// - `NotUnicast`: if `addr` or `hardware_addr` is not unicast. The cache
    ///   is left unchanged.
    pub fn insert(
        &self,
        iface: IfaceHandle,
        addr: IpAddr,
        hardware_addr: HardwareAddress,
        expires_at: Instant,
    ) -> Result<(), NotUnicast> {
        self.stack.with(|i| {
            wake_if_ok(
                i.stack
                    .neighbor_cache_mut()
                    .insert(iface, addr, hardware_addr, instant_to_xarxa(expires_at)),
            )
        })
    }

    /// Remove the entry for a neighbor, returning it if there was one.
    ///
    /// Removing an entry whose resolution is still in progress drops the packets
    /// parked on it at the next poll.
    pub fn remove(&self, iface: IfaceHandle, addr: IpAddr) -> Option<Neighbor> {
        self.stack
            .with(|i| (i.stack.neighbor_cache_mut().remove(iface, addr), NoWake))
            .map(Neighbor::from_xarxa)
    }

    /// Keep only the entries for which `f` returns true.
    ///
    /// Same caveat as [`NeighborCache::remove`] for entries being resolved.
    pub fn retain(&self, mut f: impl FnMut(&Neighbor) -> bool) {
        self.stack.with(|i| {
            (
                i.stack.neighbor_cache_mut().retain(|n| f(&Neighbor::from_xarxa(*n))),
                NoWake,
            )
        })
    }

    /// Remove all entries for one interface.
    ///
    /// Same caveat as [`NeighborCache::remove`] for entries being resolved.
    pub fn clear_iface(&self, iface: IfaceHandle) {
        self.stack
            .with(|i| (i.stack.neighbor_cache_mut().clear_iface(iface), NoWake))
    }

    /// Remove all entries.
    ///
    /// Same caveat as [`NeighborCache::remove`] for entries being resolved.
    pub fn clear(&self) {
        self.stack.with(|i| (i.stack.neighbor_cache_mut().clear(), NoWake))
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.stack.with(|i| (i.stack.neighbor_cache().len(), NoWake))
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.stack.with(|i| (i.stack.neighbor_cache().is_empty(), NoWake))
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use super::*;

    // Compile-time checks of the public API. Runtime tests of the cache need a
    // `Stack` and a mock driver, which these checks do not build.

    fn assert_value_type<T: Copy + Clone + Debug + PartialEq + Eq>() {}

    #[test]
    fn entry_types_are_plain_values() {
        assert_value_type::<Neighbor>();
        assert_value_type::<NeighborState>();
    }

    #[test]
    fn cache_handle_is_copy() {
        fn assert_copy<T: Copy + Clone>() {}
        assert_copy::<NeighborCache<'static>>();
    }

    /// Never called: it only has to compile. It pins the signature of every public
    /// method, so that a change to one of them shows up here.
    #[allow(dead_code)]
    fn api_signatures(cache: NeighborCache<'static>, iface: IfaceHandle, addr: IpAddr, hw: HardwareAddress) {
        let _: Option<Neighbor> = cache.get(iface, addr);
        let _: Option<Neighbor> = cache.remove(iface, addr);
        let _: Result<(), NotUnicast> = cache.insert(iface, addr, hw, Instant::from_millis(0));
        let _: usize = cache.len();
        let _: bool = cache.is_empty();
        cache.for_each(|_n: Neighbor| {});
        cache.retain(|_n: &Neighbor| true);
        cache.clear_iface(iface);
        cache.clear();
        for _n in cache.iter() {}
    }
}