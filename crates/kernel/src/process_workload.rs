//! Trusted verification caller, separate from process lifetime and scheduling policy.
use crate::{cpu, memory, percpu, platform::config, process, time};
use cpu::context::{Context, ESR_DATA_ABORT_LOWER, ESR_SVC64};
use process::{Error, Origin, ProcessId, Reason, Registry, Spec, State};
const EXIT_CODE: u64 = 0x4b4f_4c31;
const NEW_EXIT_CODE: u64 = EXIT_CODE + 1;
const MODE_EXIT: u64 = 0;
const MODE_FAULT: u64 = 1;
const MODE_DENIED_CREATE: u64 = 2;
const TIMER_SLICES: u64 = 1;
const ENTRY_ARGUMENTS: usize = 5;
const STRESS_CYCLES: usize = 32;
const VERIFY_TIMEOUT: time::Duration = time::Duration::from_secs(2);
unsafe extern "C" {
    static lifecycle_image_start: u8;
    static lifecycle_image_end: u8;
}
fn image() -> &'static [u8] {
    let start = &raw const lifecycle_image_start as usize;
    let end = &raw const lifecycle_image_end as usize;
    assert!(end > start && end - start <= config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: immutable trusted linker extents, checked bounded size.
    unsafe { core::slice::from_raw_parts(start as *const u8, end - start) }
}
fn spec(owner: usize, mode: u64, code: u64) -> Spec<'static> {
    let mut context = Context::ZERO;
    context.pc = memory::USER_CODE as u64;
    context.sp = memory::USER_STACK_TOP as u64;
    context.gpr[..ENTRY_ARGUMENTS].copy_from_slice(&[
        code,
        memory::USER_DATA as u64,
        mode,
        TIMER_SLICES,
        memory::USER_GUARD as u64,
    ]);
    Spec {
        image: image(),
        context,
        owner,
        entry: memory::USER_CODE,
        slice_limit: None,
    }
}
fn create(registry: &mut Registry, p: &mut memory::Physical, spec: Spec<'_>) -> ProcessId {
    registry
        .create(p, Origin::Bootstrap, spec, None)
        .unwrap_or_else(|f| process::reject(f.error))
}
fn tag(registry: &Registry, id: ProcessId) -> u64 {
    assert_eq!(registry.state(id), Ok(State::Completed));
    assert!(crate::scheduler::detached(id));
    // SAFETY: INV-USER-RETIRE: retained private space, acquired scheduler unlink,
    // all CPU writers quiescent; inspection ends before explicit reclamation.
    unsafe { core::ptr::read_volatile(registry.data_address(id).unwrap() as *const u64) }
}
pub fn exercise(
    p: &mut memory::Physical,
    registry: &mut Registry,
    mut report: impl FnMut(&str, bool),
) {
    percpu::primary_only();
    cpu::mask();
    #[cfg(feature = "kernel-tests")]
    {
        let duplicate = Registry::try_new().err() == Some(Error::AlreadyOwned);
        // SAFETY: INV-REMOTE-READER: control carries no pointer; CPU1 returns only
        // its observed coordinator rejection, and CPU0 waits before reading it.
        unsafe {
            crate::smp::experiment::submit(crate::smp::experiment::PROCESS_CONTEXT, 0);
        }
        crate::smp::experiment::complete(crate::smp::experiment::PROCESS_CONTEXT);
        report(
            "process_registry_and_cpu_ownership",
            duplicate
                && crate::smp::experiment::RESULT.load(core::sync::atomic::Ordering::Acquire) == 1,
        );
    }
    let before = p.available();
    let first = create(registry, p, spec(percpu::BOOT_CPU, MODE_EXIT, EXIT_CODE));
    report(
        "process_preparation_and_admission",
        registry.state(first) == Ok(State::Prepared)
            && registry.completion(first) == Err(Error::Transition),
    );
    #[cfg(feature = "process-reclaim-negative")]
    registry
        .reclaim(p, first)
        .unwrap_or_else(|error| process::reject(error));
    registry.start(first).unwrap();
    #[cfg(feature = "process-start-negative")]
    registry
        .start(first)
        .unwrap_or_else(|error| process::reject(error));
    let fault = create(
        registry,
        p,
        spec(percpu::SECONDARY_CPU, MODE_FAULT, EXIT_CODE),
    );
    registry.start(fault).unwrap();
    registry.dispatch(Some(VERIFY_TIMEOUT));
    #[cfg(feature = "process-exit-negative")]
    registry
        .repeat_completion(first)
        .unwrap_or_else(|error| process::reject(error));
    let old_completion = registry.completion(first).unwrap();
    report(
        "process_normal_exit_and_fault",
        old_completion.reason == Reason::Exited(EXIT_CODE)
            && registry.completion(fault).unwrap().reason
                == Reason::Faulted {
                    class: ESR_DATA_ABORT_LOWER,
                    address: memory::USER_GUARD,
                }
            && tag(registry, first) == EXIT_CODE
            && tag(registry, fault) == EXIT_CODE,
    );
    let live_pages = p.available();
    let mut rollback = true;
    for owner in 0..config::ACTIVE_CPUS {
        for step in kernel_core::process::CREATION_STEPS {
            let failure = registry
                .create(
                    p,
                    Origin::Bootstrap,
                    spec(owner, MODE_EXIT, EXIT_CODE),
                    Some(step),
                )
                .expect_err("injected creation failure");
            if p.available() != live_pages {
                process::reject(Error::RollbackLeak);
            }
            rollback &= failure.error == Error::Allocation
                && failure.completion.is_some_and(|c| {
                    c.reason == Reason::CreationFailed(step)
                        && registry.state(c.id) == Err(Error::Stale)
                })
                && p.available() == live_pages
                && registry.live() == 2;
        }
    }
    report("process_creation_rollback", rollback);
    let mut ids: [Option<ProcessId>; config::USER_PROCESSES] = [None; config::USER_PROCESSES];
    for owner in 0..config::ACTIVE_CPUS {
        for slot in 0..config::USER_PROCESSES_PER_CPU - 1 {
            ids[owner * config::USER_PROCESSES_PER_CPU + slot] =
                Some(create(registry, p, spec(owner, MODE_EXIT, EXIT_CODE)));
        }
    }
    let full_pages = p.available();
    let exhausted = (0..config::ACTIVE_CPUS).all(|owner| {
        registry
            .create(
                p,
                Origin::Bootstrap,
                spec(owner, MODE_EXIT, EXIT_CODE),
                None,
            )
            .err()
            .is_some_and(|f| f.error == Error::Capacity && f.completion.is_none())
    });
    report(
        "process_capacity_exhaustion",
        exhausted && p.available() == full_pages && registry.live() == config::USER_PROCESSES,
    );
    for id in ids.iter().flatten() {
        registry.start(*id).unwrap();
    }
    registry.dispatch(Some(VERIFY_TIMEOUT));
    for id in ids.iter().flatten() {
        registry.reclaim(p, *id).unwrap();
    }
    registry.reclaim(p, first).unwrap();
    registry.reclaim(p, fault).unwrap();
    let new = create(
        registry,
        p,
        spec(percpu::BOOT_CPU, MODE_EXIT, NEW_EXIT_CODE),
    );
    #[cfg(feature = "process-stale-negative")]
    registry
        .start(first)
        .unwrap_or_else(|error| process::reject(error));
    let generation_ok = new.slot() == first.slot()
        && new.generation() > first.generation()
        && registry.start(first) == Err(Error::Stale)
        && registry.data_address(first) == Err(Error::Stale)
        && registry.validate_completion(old_completion) == Err(Error::Stale);
    registry.start(new).unwrap();
    registry.dispatch(Some(VERIFY_TIMEOUT));
    report(
        "process_slot_generation_reuse",
        generation_ok
            && registry.completion(new).unwrap().reason == Reason::Exited(NEW_EXIT_CODE)
            && tag(registry, new) == NEW_EXIT_CODE,
    );
    #[cfg(feature = "kernel-tests")]
    assert_eq!(
        registry.repeat_completion(new),
        Err(Error::Transition),
        "double exit accepted"
    );
    let duplicate_start = registry.start(new) == Err(Error::Transition);
    // Completion is single-publication; a second table terminal transition is tested
    // in the shared host protocol, and a completed process is never dispatched again.
    let saved = registry.completion(new).unwrap();
    registry.dispatch(Some(VERIFY_TIMEOUT));
    let terminal_stable = registry.completion(new) == Ok(saved);
    registry.reclaim(p, new).unwrap();
    report(
        "process_terminal_rejections",
        duplicate_start && terminal_stable && registry.reclaim(p, new) == Err(Error::Stale),
    );
    let denied = registry
        .create(p, Origin::El0, spec(0, MODE_EXIT, EXIT_CODE), None)
        .err()
        .is_some_and(|f| f.error == Error::Unauthorized && f.completion.is_none());
    let invalid = registry
        .create(
            p,
            Origin::Bootstrap,
            Spec {
                image: &[],
                ..spec(0, MODE_EXIT, EXIT_CODE)
            },
            None,
        )
        .err()
        .is_some_and(|f| f.error == Error::InvalidImage);
    let denied_process = create(registry, p, spec(0, MODE_DENIED_CREATE, EXIT_CODE));
    registry.start(denied_process).unwrap();
    registry.dispatch(Some(VERIFY_TIMEOUT));
    report(
        "process_authority_boundary",
        denied
            && invalid
            && registry.completion(denied_process).unwrap().reason
                == Reason::Faulted {
                    class: ESR_SVC64,
                    address: 0,
                }
            && registry.live() == 1,
    );
    registry.reclaim(p, denied_process).unwrap();
    // Secondary-only work proves empty CPU0 queues and fixed owner execution.
    let sparse = create(
        registry,
        p,
        spec(percpu::SECONDARY_CPU, MODE_EXIT, EXIT_CODE),
    );
    registry.start(sparse).unwrap();
    registry.dispatch(None);
    report(
        "process_sparse_affinity",
        sparse.slot() / config::USER_PROCESSES_PER_CPU == percpu::SECONDARY_CPU
            && registry.completion(sparse).unwrap().reason == Reason::Exited(EXIT_CODE),
    );
    registry.reclaim(p, sparse).unwrap();
    let mut stress = true;
    for cycle in 0..STRESS_CYCLES {
        let code = EXIT_CODE + cycle as u64;
        let left = create(registry, p, spec(percpu::BOOT_CPU, MODE_EXIT, code));
        let right = create(
            registry,
            p,
            spec(
                percpu::SECONDARY_CPU,
                if cycle % 2 == 0 {
                    MODE_EXIT
                } else {
                    MODE_FAULT
                },
                code,
            ),
        );
        registry.start(left).unwrap();
        registry.start(right).unwrap();
        registry.dispatch(Some(VERIFY_TIMEOUT));
        stress &= registry.completion(left).unwrap().reason == Reason::Exited(code)
            && tag(registry, left) == code
            && tag(registry, right) == code;
        let expected = if cycle % 2 == 0 {
            Reason::Exited(code)
        } else {
            Reason::Faulted {
                class: ESR_DATA_ABORT_LOWER,
                address: memory::USER_GUARD,
            }
        };
        stress &= registry.completion(right).unwrap().reason == expected;
        registry.reclaim(p, left).unwrap();
        registry.reclaim(p, right).unwrap();
        stress &= registry.live() == 0 && p.available() == before;
    }
    report("process_bounded_stress", stress);
    report(
        "process_resource_reclamation",
        registry.live() == 0 && p.available() == before,
    );
}
