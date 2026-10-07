#![no_std]
#![no_main]
use native_userspace::native::{self, ClockSnapshot};
native_userspace::entry!();

// External ABI/measurement actor, not an alternative production application.
const MAGIC: u64 = 0x434c_4b01;
const WARMUPS: usize = 4;
const SAMPLES: usize = 24;
const TOTAL: usize = WARMUPS + SAMPLES;
const ZERO: ClockSnapshot = ClockSnapshot {
    ticks: 0,
    frequency: 0,
    execution_window_ticks: 0,
    read_window_service_ticks: 0,
};
fn ordered(a: ClockSnapshot, b: ClockSnapshot) -> bool {
    a.ticks <= b.ticks
        && a.execution_window_ticks <= b.execution_window_ticks
        && a.read_window_service_ticks <= b.read_window_service_ticks
}
fn failure(mode: u64, index: usize, reason: u64, snapshots: [ClockSnapshot; 3]) -> ! {
    // A partial schema-0 diagnostic is never a schema-1 performance observation.
    // Called only outside the measured interval; raw failure context is retained.
    for word in [MAGIC, 0, mode, index as u64, reason] {
        native::report(word);
    }
    for snapshot in snapshots {
        for word in [
            snapshot.ticks,
            snapshot.frequency,
            snapshot.execution_window_ticks,
            snapshot.read_window_service_ticks,
        ] {
            native::report(word);
        }
    }
    native::exit(200 + reason)
}
#[unsafe(no_mangle)]
pub extern "C" fn native_main(_receiver: u64, _feedback: u64, mode: u64) -> ! {
    if !matches!(mode, 1 | 2) {
        failure(mode, 0, 1, [ZERO; 3]);
    }
    let mut report = [0u64; 64];
    let mut recorded_queries = [ZERO; TOTAL];
    let mut frequency = 0;
    let mut previous = None;
    for (index, recorded_query) in recorded_queries.iter_mut().enumerate() {
        // Exactly the same three CLOCK calls in both modes. The envelope includes
        // endpoint probes and a useful CLOCK query, not exclusive syscall CPU cost.
        let start = native::clock_snapshot();
        let query = native::clock_snapshot();
        if mode == 2 {
            // SAFETY: initialized aligned actor-private cell, exclusively borrowed
            // and live for this write. Volatile retains the named ON recorder cost.
            unsafe {
                core::ptr::write_volatile(recorded_query, query);
            }
        }
        let end = native::clock_snapshot();
        let snapshots = [start, query, end];
        if index == 0 {
            frequency = start.frequency;
        }
        if frequency == 0 || snapshots.iter().any(|s| s.frequency != frequency) {
            failure(mode, index, 2, snapshots);
        }
        if !ordered(start, query)
            || !ordered(query, end)
            || previous.is_some_and(|last| !ordered(last, start))
        {
            failure(mode, index, 3, snapshots);
        }
        let Some(wall) = end.ticks.checked_sub(start.ticks) else {
            failure(mode, index, 4, snapshots);
        };
        let Some(window) = end
            .execution_window_ticks
            .checked_sub(start.execution_window_ticks)
        else {
            failure(mode, index, 5, snapshots);
        };
        let Some(read_window) = end
            .read_window_service_ticks
            .checked_sub(start.read_window_service_ticks)
        else {
            failure(mode, index, 6, snapshots);
        };
        if read_window != 0 {
            failure(mode, index, 7, snapshots);
        }
        if wall
            .checked_sub(window)
            .and_then(|rest| rest.checked_sub(read_window))
            .is_none()
        {
            failure(mode, index, 8, snapshots);
        }
        // Identical endpoint storage and useful-query oracle after end in both modes.
        report[8 + index] = wall;
        report[36 + index] = window;
        previous = Some(end);
    }
    report[..8].copy_from_slice(&[
        MAGIC,
        1,
        mode,
        frequency,
        WARMUPS as u64,
        SAMPLES as u64,
        TOTAL as u64,
        0,
    ]);
    // Existing fixed-capacity channel: all 64 words are emitted outside envelopes.
    for word in report {
        native::report(word);
    }
    native::exit(0)
}
