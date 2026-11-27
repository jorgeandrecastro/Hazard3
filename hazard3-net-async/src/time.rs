//! Conversions between `embassy-time` and `xarxa` time types.
//!
//! xarxa instants are the low 32 bits of the millisecond count, so they wrap around
//! every ~49.7 days and only compare correctly when they are close to each other.
//! Conversions therefore go through an offset from the current time, which saturates
//! at `xarxa::time::Duration::MAX`.

use embassy_time::{Duration, Instant};
use xarxa::time::{Duration as XarxaDuration, Instant as XarxaInstant};

/// The xarxa instant for `now`.
#[inline]
pub(crate) fn now_to_xarxa(now: Instant) -> XarxaInstant {
    XarxaInstant::from_millis(now.as_millis() as u32)
}

/// Convert an instant, going through its offset from the current time.
///
/// Offsets saturate at `xarxa::time::Duration::MAX` in either direction.
pub(crate) fn instant_to_xarxa(instant: Instant) -> XarxaInstant {
    instant_to_xarxa_at(Instant::now(), instant)
}

/// Convert an instant, going through its offset from the current time.
pub(crate) fn instant_from_xarxa(instant: XarxaInstant) -> Instant {
    instant_from_xarxa_at(Instant::now(), instant)
}

/// [`instant_to_xarxa`] with `now` given, so that it does not read the clock.
fn instant_to_xarxa_at(now: Instant, instant: Instant) -> XarxaInstant {
    let xnow = now_to_xarxa(now);
    if instant >= now {
        xnow + millis_to_xarxa(instant.as_millis() - now.as_millis())
    } else {
        xnow - millis_to_xarxa(now.as_millis() - instant.as_millis())
    }
}

/// [`instant_from_xarxa`] with `now` given, so that it does not read the clock.
fn instant_from_xarxa_at(now: Instant, instant: XarxaInstant) -> Instant {
    let xnow = now_to_xarxa(now);
    match instant.checked_duration_since(xnow) {
        Some(ahead) => now + duration_from_xarxa(ahead),
        None => {
            let behind = duration_from_xarxa(xnow.duration_since(instant));
            now.checked_sub(behind).unwrap_or(Instant::MIN)
        }
    }
}

/// Milliseconds to a xarxa duration, saturating at `XarxaDuration::MAX`.
///
/// Saturating at `u32::MAX` would be wrong: `XarxaDuration::MAX` is smaller, and an
/// offset beyond it makes instants compare the wrong way round.
#[inline]
fn millis_to_xarxa(millis: u64) -> XarxaDuration {
    let max = XarxaDuration::MAX.as_millis() as u64;
    XarxaDuration::from_millis(millis.min(max) as u32)
}

/// Convert a duration, rounding up to the next millisecond and saturating at
/// `xarxa::time::Duration::MAX`.
// Only some features use it (for example TCP timeouts), so a build without them
// would warn.
#[allow(dead_code)]
pub(crate) fn duration_to_xarxa(duration: Duration) -> XarxaDuration {
    millis_to_xarxa(duration.as_micros().div_ceil(1000))
}

#[inline]
pub(crate) fn duration_from_xarxa(duration: XarxaDuration) -> Duration {
    Duration::from_millis(duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Past the first wrap of the 32-bit millisecond count (2^32 ms ~ 49.7 days), and
    /// more than 90 days after the epoch, so that the tests can go 90 days back.
    const NOW_MS: u64 = 20_000_000_000;

    fn now() -> Instant {
        Instant::from_millis(NOW_MS)
    }

    fn round_trip(t: Instant) -> Instant {
        instant_from_xarxa_at(now(), instant_to_xarxa_at(now(), t))
    }

    #[test]
    fn now_maps_to_itself() {
        assert_eq!(round_trip(now()), now());
    }

    #[test]
    fn near_instants_round_trip_in_both_directions() {
        for delta in [1, 1_000, 60_000, 3_600_000, 86_400_000] {
            let future = Instant::from_millis(NOW_MS + delta);
            let past = Instant::from_millis(NOW_MS - delta);
            assert_eq!(round_trip(future), future, "+{delta} ms");
            assert_eq!(round_trip(past), past, "-{delta} ms");
        }
    }

    #[test]
    fn round_trip_survives_the_32_bit_wrap() {
        // The low 32 bits of `now` are far from the high 32 bits: the offset must
        // be computed on the full 64-bit values.
        assert!(NOW_MS > u32::MAX as u64);
        let t = Instant::from_millis(NOW_MS + 5_000);
        assert_eq!(round_trip(t), t);
    }

    #[test]
    fn far_future_saturates_at_the_max_offset() {
        let far = Instant::from_millis(NOW_MS + 90 * 86_400_000);
        let max = now() + duration_from_xarxa(XarxaDuration::MAX);
        assert_eq!(round_trip(far), max);
    }

    #[test]
    fn far_past_saturates_at_the_max_offset() {
        let far = Instant::from_millis(NOW_MS - 90 * 86_400_000);
        let min = now() - duration_from_xarxa(XarxaDuration::MAX);
        assert_eq!(round_trip(far), min);
    }

    #[test]
    fn past_before_the_epoch_is_clamped() {
        // `now` is 1 s after the epoch, and the xarxa instant is 5 s in the past:
        // there is no such `Instant`, so the result is `Instant::MIN`, with no panic.
        let now = Instant::from_millis(1_000);
        let five_s_ago = now_to_xarxa(now) - XarxaDuration::from_millis(5_000);
        assert_eq!(instant_from_xarxa_at(now, five_s_ago), Instant::MIN);
    }

    #[test]
    fn durations_round_trip_in_whole_milliseconds() {
        for ms in [0, 1, 1_500, 60_000] {
            let d = Duration::from_millis(ms);
            assert_eq!(duration_from_xarxa(duration_to_xarxa(d)), d);
        }
    }

    #[test]
    fn huge_durations_saturate() {
        let huge = Duration::from_secs(365 * 86_400);
        assert_eq!(duration_to_xarxa(huge), XarxaDuration::MAX);
    }
}