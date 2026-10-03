#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
use core::arch::{asm, global_asm};
use kernel_core::{
    execution as abi,
    window::{self, Backend, Reduction, Span},
};
use routing::{Consumer, Input, Profile, Route};
include!(concat!(env!("OUT_DIR"), "/profile.rs"));

// INV-DEMO-ENTRY: trusted fixed-address RX image, native supervisor supplies a private
// aligned guarded stack and CPU-owned task ID in x0. No shared writable globals/BSS.
global_asm!(".section .text.entry,\"ax\"\n.global _start\n_start:\n bl user_main\n b .");
#[cfg(feature = "evidence")]
const REPORT_HEADER: u64 = 0x4b56_5233; // KVR3 includes exclusive EL0/native service counters.
#[cfg(feature = "dev")]
const REPORT_BENCH: u64 = 0x4245_4e43;
#[cfg(feature = "evidence")]
const REPORT_FINAL: u64 = 0x444f_4e45;
#[cfg(feature = "evidence")]
const MIN_CAPTURE_SLICES: u64 = 3;
#[cfg(feature = "dev")]
const WARMUP: usize = 16;
#[cfg(feature = "dev")]
const SAMPLES: usize = 128;
const WORKLOAD_START: u32 = 0;
const WORKLOAD_END: u32 = 2;

fn svc<const OP: u16>(first: u64, second: u64) -> [u64; abi::REPLY_REGISTERS] {
    let (mut a, mut b, mut c, mut d, mut e, mut f) = (first, second, 0u64, 0u64, 0u64, 0u64);
    // SAFETY: INV-DEMO-SVC: nonpointer fixed-width native arguments, complete register
    // clobbers; kernel uses the current task, validates bounds, and retains no user alias.
    unsafe {
        asm!("svc {op}", op = const OP, inout("x0") a, inout("x1") b,
        inout("x2") c, inout("x3") d, inout("x4") e, inout("x5") f, options(nostack));
    }
    [a, b, c, d, e, f]
}
#[cfg(feature = "evidence")]
fn emit(value: u64) {
    assert_eq!(svc::<{ abi::REPORT }>(value, 0)[0], abi::OK);
}
fn finish(value: u64) -> ! {
    let _ = svc::<{ abi::FINISH }>(value, 0);
    loop {
        core::hint::spin_loop();
    }
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    finish(0)
}
struct Native;
impl Backend for Native {
    fn reduce(&mut self, span: Span) -> Result<Reduction, window::Error> {
        let reply = svc::<{ abi::READ_WINDOW }>(span.start as u64, span.end as u64);
        if reply[0] != abi::OK {
            return Err(window::Error::Bounds);
        }
        let words = usize::try_from(reply[1]).map_err(|_| window::Error::Bounds)?;
        if words > abi::HISTORY_WORDS {
            return Err(window::Error::Bounds);
        }
        let mut data = [0u32; abi::HISTORY_WORDS];
        for (index, value) in data.iter_mut().enumerate() {
            *value = (reply[abi::DATA_REGISTER_START + index / abi::VALUES_PER_REGISTER]
                >> ((index % abi::VALUES_PER_REGISTER) * u32::BITS as usize))
                as u32;
        }
        assert!(data[words..].iter().all(|&value| value == 0));
        window::reduce(
            &data[..words],
            Span {
                start: 0,
                end: words as u32,
            },
        )
    }
}
fn input<'a>(route: Route, inclusive: &'a [u8; 4], counted: &'a [u8; 8]) -> Input<'a> {
    match route {
        Route::Native => Input::Native(Span {
            start: WORKLOAD_START,
            end: WORKLOAD_END,
        }),
        Route::Inclusive => Input::Encoded(inclusive),
        Route::Counted | Route::EmptyFirst => Input::Encoded(counted),
    }
}
fn invoke(consumer: &mut Consumer) -> (Reduction, routing::Work) {
    let inclusive = [0, 0, 1, 0]; // LE16 inclusive [0,1].
    let counted = [0, 0, 0, 0, 0, 0, 0, 2]; // BE32 start=0,count=2.
    let request = input(consumer.route(), &inclusive, &counted);
    invoke_prepared(consumer, request)
}
fn invoke_prepared(consumer: &mut Consumer, request: Input<'_>) -> (Reduction, routing::Work) {
    consumer
        .call_with(&mut Native, core::hint::black_box(request))
        .unwrap()
}
#[cfg(feature = "dev")]
fn benchmark(mut consumer: Consumer, oracle: Reduction) {
    let route = consumer.route();
    let inclusive = [0, 0, 1, 0];
    let counted = [0, 0, 0, 0, 0, 0, 0, 2];
    let mut warmup = [0u64; WARMUP];
    for value in &mut warmup {
        let request = input(route, &inclusive, &counted);
        let start = svc::<{ abi::CLOCK }>(0, 0)[0];
        assert_eq!(invoke_prepared(&mut consumer, request).0, oracle);
        *value = svc::<{ abi::CLOCK }>(0, 0)[0] - start;
    }
    emit(REPORT_BENCH);
    emit(route as u64);
    emit(SAMPLES as u64);
    for value in warmup {
        emit(value);
    }
    let slices_before = svc::<{ abi::SLICES }>(0, 0)[0];
    let mut samples = [0u64; SAMPLES];
    let mut route_el0_cpu_samples = [0u64; SAMPLES];
    let mut native_service_cpu_samples = [0u64; SAMPLES];
    for (sample_index, value) in samples.iter_mut().enumerate() {
        let request = input(route, &inclusive, &counted);
        let start_accounting = svc::<{ abi::CLOCK }>(0, 0);
        let (result, _) = invoke_prepared(&mut consumer, request);
        let end_accounting = svc::<{ abi::CLOCK }>(0, 0);
        let end = end_accounting[0];
        assert_eq!(result, oracle);
        *value = end.checked_sub(start_accounting[0]).unwrap();
        route_el0_cpu_samples[sample_index] =
            end_accounting[2].checked_sub(start_accounting[2]).unwrap();
        native_service_cpu_samples[sample_index] =
            end_accounting[3].checked_sub(start_accounting[3]).unwrap();
    }
    let slices_after = svc::<{ abi::SLICES }>(0, 0)[0];
    for value in samples {
        emit(value);
    }
    for value in route_el0_cpu_samples {
        emit(value);
    }
    for value in native_service_cpu_samples {
        emit(value);
    }
    emit(slices_after - slices_before);
    let c = consumer.counters(route);
    for value in [
        c.native_calls,
        c.compat_calls,
        c.backend_calls,
        c.translations,
        c.copies,
        c.copied_bytes,
        c.conversions,
        c.errors,
        c.fallbacks,
    ] {
        emit(value);
    }
}
#[unsafe(no_mangle)]
extern "C" fn user_main(id: u64) -> ! {
    #[cfg(feature = "profile-negative")]
    let profile_bytes = {
        let mut bytes = PROFILE;
        bytes[0] ^= 1;
        bytes
    };
    #[cfg(not(feature = "profile-negative"))]
    let profile_bytes = PROFILE;
    let profile = Profile::decode(&profile_bytes, EXPECTED).unwrap();
    #[cfg(feature = "adapter-negative")]
    if id == 1 {
        let _ = svc::<{ u16::MAX }>(0, 0);
    }
    let slot = id as usize % routing::CONSUMERS;
    let mut consumer = Consumer::from_profile(&profile, slot).unwrap();
    #[cfg(not(feature = "evidence"))]
    {
        assert_eq!(svc::<{ abi::CAPTURE }>(0, 0)[0], abi::OK);
        let _ = invoke(&mut consumer);
        finish(1);
    }
    #[cfg(feature = "evidence")]
    {
        let mut checks = 0;
        routing::conformance::run(|_, passed| {
            assert!(passed);
            checks += 1;
        });
        while svc::<{ abi::SLICES }>(0, 0)[0] < MIN_CAPTURE_SLICES {
            core::hint::spin_loop();
        }
        assert_eq!(svc::<{ abi::READ_WINDOW }>(0, 1)[0], abi::NOT_CAPTURED);
        assert_eq!(svc::<{ abi::CAPTURE }>(0, 0)[0], abi::OK);
        assert_ne!(svc::<{ abi::READ_WINDOW }>(u64::MAX, 1)[0], abi::OK);
        assert_ne!(
            svc::<{ abi::READ_WINDOW }>(0, abi::HISTORY_WORDS as u64 + 1)[0],
            abi::OK
        );
        let mut denied = Consumer::from_profile(&Profile::native(), slot).unwrap();
        assert_eq!(
            denied.call_with(
                &mut Native,
                Input::Native(Span {
                    start: 0,
                    end: u32::MAX
                })
            ),
            Err(routing::Error::Bounds)
        );
        #[cfg(feature = "dev")]
        assert_eq!(denied.counters(Route::Native).errors, 1);
        let oracle = Native
            .reduce(Span {
                start: WORKLOAD_START,
                end: WORKLOAD_END,
            })
            .unwrap();
        assert!(oracle.sum > 0);
        let (result, work) = invoke(&mut consumer);
        assert_eq!(result, oracle);
        assert_eq!(Native.reduce(Span { start: 0, end: 0 }).unwrap().sum, 0);
        #[cfg(feature = "bug")]
        {
            let p = Profile::new(1, [Route::EmptyFirst; routing::CONSUMERS]).unwrap();
            let mut historical = Consumer::from_profile(&p, slot).unwrap();
            let old = historical
                .call_with(&mut Native, Input::Encoded(&[0; 8]))
                .unwrap()
                .0;
            assert_eq!(old, Native.reduce(Span { start: 0, end: 1 }).unwrap());
            assert_eq!(old.words, 1);
        }
        emit(REPORT_HEADER);
        emit(id);
        emit(consumer.route() as u64);
        emit(profile.generation() as u64);
        emit(checks);
        emit(oracle.sum);
        emit(oracle.words as u64);
        emit(svc::<{ abi::CLOCK }>(0, 0)[1]);
        emit(work.translations + u64::from(cfg!(feature = "report-negative")));
        emit(work.copied_bytes);
        emit(work.backend_calls);
        for chunk in EXPECTED.as_chunks::<{ core::mem::size_of::<u64>() }>().0 {
            emit(u64::from_le_bytes(*chunk));
        }
        #[cfg(feature = "dev")]
        {
            let saved = consumer.route();
            let mut tx = consumer.begin().unwrap();
            assert_eq!(tx.try_switch(Route::Native), Err(routing::Error::Busy));
            drop(tx);
            consumer.switch(Route::Native).unwrap();
            assert_eq!(invoke(&mut consumer).0, oracle);
            consumer.switch(saved).unwrap();
            let empty = Native.reduce(Span { start: 0, end: 0 }).unwrap();
            assert_eq!(empty.sum, 0);
            let mut order = Route::ALL;
            if id & 1 != 0 {
                order.reverse();
            }
            for route in order {
                if route.available() {
                    let p = Profile::new(1, [route; routing::CONSUMERS]).unwrap();
                    benchmark(Consumer::from_profile(&p, slot).unwrap(), oracle);
                }
            }
        }
        emit(REPORT_FINAL);
        finish(1)
    }
}
