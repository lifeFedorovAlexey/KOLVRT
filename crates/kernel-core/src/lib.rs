#![no_std]
#![forbid(unsafe_code)]

pub mod execution;
pub mod memory;
pub mod platform;
pub mod scheduling;
pub mod time;
pub mod window;

/// Nearest-rank quantiles; samples are timer ticks, not processor cycles.
pub fn quantiles(samples: &mut [u64]) -> Option<[u64; 3]> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_unstable();
    let rank = |p: usize| samples[(samples.len() * p).div_ceil(100) - 1];
    Some([rank(50), rank(95), rank(99)])
}
