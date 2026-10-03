//! Trusted, bounded, pure host workloads. No deployment, network or persistent writes.
#![forbid(unsafe_code)]
use kernel_core::window::Span;
use routing::{Consumer, Input, Profile, Route};
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const ITERATIONS: u64 = 50000;
const WARMUP: u64 = 5000;
const MIN_WARMUP_BATCHES: usize = 3;
const MAX_WARMUP_BATCHES: usize = 20;
const STABILIZATION_PERCENT: u128 = 20;
const WINDOW_WORDS: u32 = 16;
const START_COUNT: u64 = 240;
pub const CONTRACT: &str = "window workload/1.0.0: immutable u32[256]; half-open windows [i%240,i%240+16); exact sum and word count; empty returns zero; invalid bounds fail; no persistent data or external effects; compat input is equivalent LE16 inclusive endpoints; native input is a half-open Span; every result is checked";
pub const PROTOCOL: &str = "physical-host-process-library-v5; 100 separate fresh-process power-pilot pairs followed by 100 fresh-process measured pairs; independence is not inferred from process creation; AB/BA alternating within each series; 3..20 warmup batches of 5000 calls; stop when last3 batch durations max-min<=20percent of min; retain every warmup batch; measured50000 useful reductions; equal input, oracle, instrumentation and inherited process limits; background load and thermal state uncontrolled; no exclusions; abort and retain evidence on first process/oracle failure; failed warmup invalidates recommendation; primary metric whole-batch wall ns excluding spawn, contracts and warmup; per-process receipts retain copied bytes and Windows PeakWorkingSetSize across process lifetime; p95 copied-byte and peak-working-set regression budgets 0 bytes; peak memory includes startup, contracts, warmup and measurement, not just the measured batch; energy unavailable and unbudgeted; one-sample blocks are a declared independence assumption and remain unvalidated; normal-approximation power plan targets90percent at expected pilot mean; not KOLVRT kernel or ARM64 behavior; predeclared bootstrap2000/seed7/confidence9500/margin1000ns/p95budget100000ns/p99unavailable";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub mode: String,
    pub checks: u64,
    pub passed: bool,
    pub checksum: u64,
    pub iterations: u64,
    pub warmup_ns: u64,
    pub warmup_batches_ns: Vec<u64>,
    pub stabilized: bool,
    pub elapsed_ns: u64,
    pub native_admissions: u64,
    pub compat_admissions: u64,
    pub copied_bytes: u64,
    pub peak_memory_bytes: Option<u64>,
    pub conversions: u64,
}

fn invoke(
    consumer: &mut Consumer,
    data: &[u32],
    span: Span,
) -> Result<kernel_core::window::Reduction, String> {
    let start = (span.start as u16).to_le_bytes();
    let end = if span.start == span.end {
        u16::MAX
    } else {
        (span.end - 1) as u16
    }
    .to_le_bytes();
    let encoded = if span.start == span.end {
        [255; 4]
    } else {
        [start[0], start[1], end[0], end[1]]
    };
    let input = if consumer.route() == Route::Native {
        Input::Native(span)
    } else {
        Input::Encoded(&encoded)
    };
    consumer
        .call(std::hint::black_box(data), std::hint::black_box(input))
        .map(|r| r.0)
        .map_err(|e| format!("{e:?}"))
}
fn consumer(native: bool) -> Consumer {
    let route = if native {
        Route::Native
    } else {
        Route::Inclusive
    };
    Consumer::from_profile(&Profile::new(1, [route; 4]).unwrap(), 0).unwrap()
}
pub fn expected_checksum() -> u64 {
    (0..ITERATIONS)
        .map(|i| {
            let start = (i % START_COUNT) as u32;
            (start..start + WINDOW_WORDS).map(u64::from).sum::<u64>()
        })
        .sum()
}
pub fn execute(native: bool, measure: bool) -> Result<Receipt, String> {
    let mut checks = 0;
    let mut passed = true;
    routing::conformance::run(|_, ok| {
        checks += 1;
        passed &= ok;
    });
    let data: Vec<_> = (0..256u32).collect();
    let mut c = consumer(native);
    passed &= invoke(&mut c, &data, Span { start: 0, end: 0 })?.sum == 0;
    checks += 1;
    if !passed {
        return Err("contract suite failed".into());
    }
    let mut receipt = Receipt {
        mode: if native { "native" } else { "compat" }.into(),
        checks,
        passed,
        checksum: 0,
        iterations: 0,
        warmup_ns: 0,
        warmup_batches_ns: vec![],
        stabilized: false,
        elapsed_ns: 0,
        native_admissions: 0,
        compat_admissions: 0,
        copied_bytes: 0,
        peak_memory_bytes: None,
        conversions: 0,
    };
    if !measure {
        return Ok(receipt);
    }
    let run = |consumer: &mut Consumer, iterations: u64| -> Result<u64, String> {
        let mut checksum = 0;
        for i in 0..iterations {
            let start = (i % START_COUNT) as u32;
            let span = Span {
                start,
                end: start + WINDOW_WORDS,
            };
            let result = invoke(consumer, &data, span)?;
            let oracle: u64 = (start..span.end).map(u64::from).sum();
            if result.sum != oracle || result.words != WINDOW_WORDS {
                return Err("workload oracle failed".into());
            }
            checksum += result.sum;
        }
        Ok(checksum)
    };
    let warmup = Instant::now();
    for _ in 0..MAX_WARMUP_BATCHES {
        let batch = Instant::now();
        run(&mut c, WARMUP)?;
        let ns: u64 = batch
            .elapsed()
            .as_nanos()
            .try_into()
            .map_err(|_| "clock overflow")?;
        receipt.warmup_batches_ns.push(ns);
        if receipt.warmup_batches_ns.len() >= MIN_WARMUP_BATCHES {
            let last =
                &receipt.warmup_batches_ns[receipt.warmup_batches_ns.len() - MIN_WARMUP_BATCHES..];
            let minimum = *last.iter().min().unwrap();
            let maximum = *last.iter().max().unwrap();
            if minimum > 0
                && u128::from(maximum - minimum) * 100
                    <= u128::from(minimum) * STABILIZATION_PERCENT
            {
                receipt.stabilized = true;
                break;
            }
        }
    }
    receipt.warmup_ns = warmup
        .elapsed()
        .as_nanos()
        .try_into()
        .map_err(|_| "clock overflow")?;
    c = consumer(native); // Counters distinguish warmup from retained useful work.
    let begin = Instant::now();
    receipt.checksum = run(&mut c, ITERATIONS)?;
    receipt.elapsed_ns = begin
        .elapsed()
        .as_nanos()
        .try_into()
        .map_err(|_| "clock overflow")?;
    receipt.iterations = ITERATIONS;
    let counters = c.counters(c.route());
    receipt.native_admissions = counters.native_calls;
    receipt.compat_admissions = counters.compat_calls;
    receipt.copied_bytes = counters.copied_bytes;
    receipt.conversions = counters.conversions;
    if counters.errors != 0 || receipt.checksum != expected_checksum() {
        return Err("accounting or checksum failure".into());
    }
    receipt.peak_memory_bytes = host_process_metrics::peak_working_set_bytes()
        .map_err(|error| format!("peak working-set measurement failed: {error}"))?;
    Ok(receipt)
}
pub fn fixture_main(native: bool) {
    let result = match std::env::args().nth(1).as_deref() {
        Some("verify") => execute(native, false),
        Some("measure") => execute(native, true),
        _ => Err("usage: fixture verify|measure".into()),
    };
    match result {
        Ok(receipt) => println!("{}", serde_json::to_string(&receipt).unwrap()),
        Err(error) => {
            eprintln!("[FAIL] migration-fixture: {error}");
            std::process::exit(1);
        }
    }
}
