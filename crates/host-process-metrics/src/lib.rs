//! Small platform boundary for process metrics used by host experiments.
#![deny(unsafe_op_in_unsafe_fn)]

use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum PeakMemoryMeasurement {
    Available { bytes: u64 },
    Unavailable { reason: PeakMemoryUnavailable },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PeakMemoryUnavailable {
    UnsupportedPlatform,
    NotEvaluated,
}

impl PeakMemoryMeasurement {
    pub fn bytes(self) -> Option<u64> {
        match self {
            Self::Available { bytes } => Some(bytes),
            Self::Unavailable { .. } => None,
        }
    }
}

/// Returns the current process' peak resident working set in bytes.
///
/// Windows reports the process peak accumulated since launch. Other targets explicitly
/// report an unsupported platform. OS acquisition errors remain errors.
pub fn peak_working_set_bytes() -> io::Result<PeakMemoryMeasurement> {
    #[cfg(windows)]
    {
        let mut counters = ProcessMemoryCountersEx {
            cb: std::mem::size_of::<ProcessMemoryCountersEx>() as u32,
            ..Default::default()
        };
        let size = counters.cb;
        // SAFETY: GetCurrentProcess returns a valid pseudo-handle. `counters` is a writable,
        // correctly sized repr(C) PROCESS_MEMORY_COUNTERS_EX buffer for this synchronous call.
        let succeeded = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
        if succeeded == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(PeakMemoryMeasurement::Available {
            bytes: counters.peak_working_set_size as u64,
        })
    }
    #[cfg(not(windows))]
    {
        Ok(PeakMemoryMeasurement::Unavailable {
            reason: PeakMemoryUnavailable::UnsupportedPlatform,
        })
    }
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
#[allow(dead_code)] // The OS fills all fields; this wrapper exposes only PeakWorkingSetSize.
struct ProcessMemoryCountersEx {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut core::ffi::c_void;
}

#[cfg(windows)]
#[link(name = "psapi")]
unsafe extern "system" {
    fn GetProcessMemoryInfo(
        process: *mut core::ffi::c_void,
        counters: *mut ProcessMemoryCountersEx,
        size: u32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::peak_working_set_bytes;

    #[cfg(windows)]
    #[test]
    fn windows_returns_a_positive_peak_for_the_live_test_process() {
        assert!(
            matches!(peak_working_set_bytes().unwrap(), super::PeakMemoryMeasurement::Available { bytes } if bytes > 0)
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_targets_report_the_specific_cause() {
        assert_eq!(
            peak_working_set_bytes().unwrap(),
            super::PeakMemoryMeasurement::Unavailable {
                reason: super::PeakMemoryUnavailable::UnsupportedPlatform
            }
        );
    }
}
