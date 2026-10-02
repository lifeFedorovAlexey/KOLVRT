use core::time::Duration;

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Convert a duration to counter ticks, rounding up so deadlines never expire early.
pub fn duration_ticks(duration: Duration, frequency: u64) -> Option<u64> {
    if frequency == 0 {
        return None;
    }
    let scaled = duration.as_nanos().checked_mul(u128::from(frequency))?;
    u64::try_from(scaled.div_ceil(NANOS_PER_SECOND)).ok()
}
#[cfg(test)]
mod tests {
    use core::time::Duration;
    const TEST_COUNTER_HZ: u64 = 62_500_000;
    #[test]
    fn rounding_and_overflow() {
        assert_eq!(
            super::duration_ticks(Duration::from_nanos(1), TEST_COUNTER_HZ),
            Some(1)
        );
        assert_eq!(
            super::duration_ticks(Duration::from_secs(1), TEST_COUNTER_HZ),
            Some(TEST_COUNTER_HZ)
        );
        assert_eq!(super::duration_ticks(Duration::from_nanos(1), 0), None);
        assert_eq!(
            super::duration_ticks(Duration::from_nanos(u64::MAX), u64::MAX),
            None
        );
        assert_eq!(super::duration_ticks(Duration::MAX, u64::MAX), None);
        assert_eq!(
            super::duration_ticks(Duration::ZERO, TEST_COUNTER_HZ),
            Some(0)
        );
        assert_eq!(
            super::duration_ticks(Duration::from_millis(1), TEST_COUNTER_HZ),
            Some(TEST_COUNTER_HZ / 1000)
        );
        assert_eq!(
            super::duration_ticks(Duration::from_secs(u64::MAX), 1),
            Some(u64::MAX)
        );
    }
}
