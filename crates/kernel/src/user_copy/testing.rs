//! Trusted EL0 fixture endpoint, compiled out of ordinary images.
use super::*;
use crate::cpu::context::Context;
const SUITE: u16 = 0x70;
const SNAPSHOT: u16 = 0x71;
const SAMPLES: usize = 32;
const WARMUP: usize = 4;
#[inline(never)]
fn input_eq(access: &Access<'_>, address: usize, length: usize, expected: &[u8]) -> bool {
    access
        .copy_from_user::<MAX_BYTES>(address, length)
        .is_ok_and(|s| s.bytes() == expected)
}
#[inline(never)]
fn input_error(access: &Access<'_>, address: usize, length: usize, expected: Error) -> bool {
    access.copy_from_user::<MAX_BYTES>(address, length).err() == Some(expected)
}
static BENCHMARKS: [[AtomicUsize; SAMPLES * 4]; crate::platform::config::ACTIVE_CPUS] =
    [const { [const { AtomicUsize::new(0) }; SAMPLES * 4] }; crate::platform::config::ACTIVE_CPUS];
pub(crate) fn call(task: &mut Task, frame: &mut Context, operation: u16) -> bool {
    if operation != SUITE && operation != SNAPSHOT {
        return false;
    }
    let address = frame.gpr[1] as usize;
    let access = Access::current(task).expect("executing copy context");
    if operation == SNAPSHOT {
        let later = access.copy_from_user::<8>(address, 8).unwrap();
        let expected = 42u64.to_le_bytes();
        #[cfg(not(feature = "user-copy-snapshot-negative"))]
        let admitted = task.copy_snapshot;
        #[cfg(feature = "user-copy-snapshot-negative")]
        let admitted: [u8; 8] = later.bytes().try_into().unwrap();
        frame.gpr[0] = u64::from(admitted == expected && later.bytes() == 43u64.to_le_bytes());
        return true;
    }
    let input = access.copy_from_user::<8>(address, 8).unwrap();
    let saved: [u8; 8] = input.bytes().try_into().unwrap();
    let stack = memory::USER_STACK_TOP - crate::platform::config::USER_STACK_PAGES * PAGE_BYTES;
    let valid = input.bytes() == 42u64.to_le_bytes()
        && access.copy_to_user(address + 16, &[0x5a; 8]) == Ok(())
        && input_eq(&access, address + 16, 8, &[0x5a; 8])
        && input_eq(&access, address + 1, 7, &[0; 7])
        && input_eq(&access, stack + PAGE_BYTES - 3, 9, &[0xa5; 9])
        && access.copy_to_user(stack + PAGE_BYTES - 3, &[0xa5; 9]) == Ok(())
        && input_eq(&access, stack, MAX_BYTES, &[0xa5; MAX_BYTES])
        && access.copy_to_user(stack, &[0xa5; MAX_BYTES]) == Ok(())
        && input_eq(&access, address, 0, &[])
        && access.copy_to_user(address, &[]) == Ok(());
    let invalid = [0, crate::platform::config::RAM_BASE, usize::MAX];
    let rejected = invalid.iter().all(|&a| {
        input_error(&access, a, 8, Error::InvalidRange)
            && access.copy_to_user(a, &[0; 8]) == Err(Error::InvalidRange)
    }) && input_error(&access, 0, 0, Error::InvalidRange)
        && input_error(&access, address, MAX_BYTES + 1, Error::TooLarge)
        && access.copy_to_user(address, &[0; MAX_BYTES + 1]) == Err(Error::TooLarge)
        && access.copy_to_user(memory::USER_CODE, &[0]) == Err(Error::PermissionDenied)
        && input_error(
            &access,
            memory::USER_GUARD,
            1,
            Error::UserFault { copied: 0 },
        )
        && input_error(
            &access,
            memory::USER_GUARD - 4,
            8,
            Error::UserFault { copied: 0 },
        )
        && access.copy_to_user(memory::USER_GUARD - 4, &[0; 8])
            == Err(Error::UserFault { copied: 0 })
        && input_error(
            &access,
            memory::UserSpace::alias((task.id() + 1) % crate::process::CAPACITY),
            8,
            Error::UserFault { copied: 0 },
        );
    // Test-only fault injection: deliberately skip page preflight for a bounded
    // range whose first four bytes are live and final four enter the guard page.
    // The production loop and recovery run unchanged, with the real current root.
    let mut poisoned = [0xcc; 8];
    let fault_address = memory::USER_GUARD - 4;
    let guard = CopyGuard::new(fault_address, poisoned.len()).unwrap();
    // SAFETY: INV-USER-COPY-TEST: current task/root retained, initialized 8-byte
    // destination; guard page deliberately faults at production LDTRB PC.
    let copied_in = unsafe { user_copy_in(poisoned.as_mut_ptr(), fault_address, poisoned.len()) };
    drop(guard);
    let guard = CopyGuard::new(fault_address, poisoned.len()).unwrap();
    // SAFETY: INV-USER-COPY-TEST: initialized byte source; only four writable user
    // bytes precede the deliberate STTRB fault, never a kernel pointer access.
    let copied_out = unsafe { user_copy_out(fault_address, poisoned.as_ptr(), poisoned.len()) };
    drop(guard);
    let faults = copied_in == 4
        && copied_out == 4
        && poisoned[4..] == [0xcc; 4]
        && !RECOVERY[percpu::id()].active.load(Ordering::Acquire);
    let mut stale = *task;
    // Same production result constructors with only page preflight omitted.
    // No partially read Snapshot may escape on the actual mid-copy fault.
    let input_atomic =
        access.read_snapshot::<8>(fault_address, 8).err() == Some(Error::UserFault { copied: 4 });
    let output_partial =
        access.write_bytes(fault_address, &[0x77; 8]) == Err(Error::UserFault { copied: 4 });
    let prefix_initialized = input_eq(&access, fault_address, 4, &[0x77; 4]);
    let empty_unmapped = access.copy_from_user::<0>(memory::USER_GUARD, 0).is_ok();
    let capacity = access.copy_from_user::<4>(address, 8).err() == Some(Error::TooLarge);
    let guard = CopyGuard::new(fault_address, 8).unwrap();
    let esr = (0x25u64 << cpu::ESR_EC_SHIFT) | 7;
    let precise = recover(esr, fault_address as u64, memory::USER_CODE as u64).is_none()
        && recover(
            esr,
            (fault_address - 1) as u64,
            &raw const user_copy_load as u64,
        )
        .is_none()
        && recover(
            esr | (1 << 6),
            fault_address as u64,
            &raw const user_copy_load as u64,
        )
        .is_none();
    drop(guard);
    stale.unlink();
    let stale_unlinked = Access::current(&stale).err() == Some(Error::StaleContext);
    let stale_generation =
        !memory::current_space(task.id(), task.process_generation() - 1, task.root());
    let terminal = {
        let mut old = *task;
        old.state = crate::scheduler::task::CONTEXT_EXITED;
        Access::current(&old).err() == Some(Error::StaleContext)
    };
    for (case, size) in [8, PAGE_BYTES, MAX_BYTES, 0].into_iter().enumerate() {
        for sample in 0..WARMUP + SAMPLES {
            let start = cpu::ticks();
            if case == 3 {
                core::hint::black_box(input_error(
                    &access,
                    memory::USER_GUARD,
                    8,
                    Error::UserFault { copied: 0 },
                ));
            } else {
                benchmark_copy(&access, stack, size);
            }
            let ticks = cpu::ticks() - start;
            if sample >= WARMUP {
                BENCHMARKS[percpu::id()][case * SAMPLES + sample - WARMUP]
                    .store(ticks as usize, Ordering::Release);
            }
        }
    }
    task.copy_snapshot = saved;
    frame.gpr[0] = u64::from(
        valid
            && rejected
            && faults
            && input_atomic
            && output_partial
            && prefix_initialized
            && empty_unmapped
            && capacity
            && precise
            && stale_unlinked
            && stale_generation
            && terminal,
    );
    true
}
#[inline(never)]
fn benchmark_copy(access: &Access<'_>, address: usize, length: usize) {
    let snapshot = access.copy_from_user::<MAX_BYTES>(address, length).unwrap();
    core::hint::black_box(snapshot.bytes());
}

unsafe extern "C" {
    static copy_image_start: u8;
    static copy_image_end: u8;
}
pub(crate) fn exercise(
    physical: &mut memory::Physical,
    registry: &mut crate::process::Registry,
    mut report: impl FnMut(&str, bool),
) {
    use crate::process::{Origin, Reason, Spec};
    let before = physical.available();
    let start = &raw const copy_image_start as usize;
    let end = &raw const copy_image_end as usize;
    assert!(end > start && end - start <= PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: immutable trusted linker image, checked page bound.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let ids: [_; crate::platform::config::ACTIVE_CPUS] = core::array::from_fn(|owner| {
        let mut context = Context::ZERO;
        context.pc = memory::USER_CODE as u64;
        context.sp = memory::USER_STACK_TOP as u64;
        let id = registry
            .create(
                physical,
                Origin::Bootstrap,
                Spec {
                    image,
                    context,
                    owner,
                    entry: memory::USER_CODE,
                    slice_limit: None,
                },
                None,
            )
            .unwrap();
        registry.start(id).unwrap();
        id
    });
    registry.dispatch(Some(crate::time::Duration::from_secs(5)));
    report(
        "user_copy_el0_boundary_and_snapshot",
        ids.iter()
            .all(|&id| registry.completion(id).unwrap().reason == Reason::Exited(1)),
    );
    for id in ids {
        registry.reclaim(physical, id).unwrap();
    }
    report(
        "user_copy_lifetime_and_reclamation",
        physical.available() == before
            && registry.live() == 0
            && ids
                .iter()
                .all(|id| !memory::live_space_generation(id.slot(), id.generation())),
    );
    for (owner, benchmarks) in BENCHMARKS.iter().enumerate() {
        #[cfg(feature = "diagnostics")]
        {
            let d = &DIAGNOSTICS[owner];
            crate::diagnostics::status(
                "DEBUG",
                "user-copy",
                format_args!(
                    "cpu={} attempts={} preflight_failures={} recovered={} last_range={:#x}+{} reason={} ESR={:#x}",
                    owner,
                    d.attempts.load(Ordering::Acquire),
                    d.failures.load(Ordering::Acquire),
                    d.recovered.load(Ordering::Acquire),
                    d.address.load(Ordering::Acquire),
                    d.length.load(Ordering::Acquire),
                    d.reason.load(Ordering::Acquire),
                    d.esr.load(Ordering::Acquire)
                ),
            );
        }
        for (case, scope) in [
            "user_copy_small",
            "user_copy_page",
            "user_copy_three_pages",
            "user_copy_failure",
        ]
        .into_iter()
        .enumerate()
        {
            let raw: [u64; SAMPLES] = core::array::from_fn(|i| {
                benchmarks[case * SAMPLES + i].load(Ordering::Acquire) as u64
            });
            let mut sorted = raw;
            let [median, p95, p99] = kernel_core::quantiles(&mut sorted).unwrap();
            crate::event!(
                "{{\"event\":\"measurement\",\"scope\":\"{}\",\"cpu\":{},\"units\":\"timer_ticks\",\"frequency\":{},\"warmup\":{},\"iterations\":{},\"median\":{},\"p95\":{},\"p99\":{},\"samples\":{:?}}}",
                scope,
                owner,
                cpu::frequency(),
                WARMUP,
                SAMPLES,
                median,
                p95,
                p99,
                raw
            );
        }
    }
}
