//! Trusted verification caller, separate from process lifetime and scheduling policy.
use crate::{cpu, memory, percpu, platform::config, process, time};
use cpu::context::{Context, ESR_DATA_ABORT_LOWER, ESR_SVC64};
use process::{Error, Origin, ProcessId, Reason, Registry, Spec, State};
const EXIT_CODE: u64 = 0x4b4f_4c31;
const NEW_EXIT_CODE: u64 = EXIT_CODE + 1;
const MODE_EXIT: u64 = 0;
const MODE_FAULT: u64 = 1;
const MODE_DENIED_CREATE: u64 = 2;
const MODE_SPIN: u64 = 3;
const MODE_WAIT: u64 = 4;
const SPIN_BUDGET: usize = 8;
const TIMER_SLICES: u64 = 1;
const ENTRY_ARGUMENTS: usize = 5;
const STRESS_CYCLES: usize = 32;
#[cfg(feature = "kernel-tests")]
const ELF_EXIT_CODE: u64 = 0x4b4f_4c31;
#[cfg(feature = "kernel-tests")]
const ELF_MAX_SLICES: usize = 16;
#[cfg(feature = "kernel-tests")]
const MINIMAL_ELF: &[u8] = include_bytes!("../assets/minimal-exit.elf");
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
        image_format: process::ImageFormat::RawFixture,
        limits: limits(),
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
    #[cfg(feature = "kernel-tests")]
    report(
        "asid_pool_exhaustion_both_cpus",
        (0..config::ACTIVE_CPUS).all(crate::asid::exhaustion_probe),
    );
    for id in ids.iter().flatten() {
        registry.start(*id).unwrap();
    }
    registry.dispatch(Some(VERIFY_TIMEOUT));
    for id in ids.iter().flatten() {
        registry.reclaim(p, *id).unwrap();
    }
    let first_data = registry.data_address(first).unwrap();
    registry.reclaim(p, first).unwrap();
    registry.reclaim(p, fault).unwrap();
    let retired_extent = p
        .pin_extent_at(
            first_data - memory::USER_DATA_FRAME_OFFSET,
            memory::USER_SPACE_PAGES,
        )
        .expect("ASID reuse witness frame");
    let new = create(
        registry,
        p,
        spec(percpu::BOOT_CPU, MODE_EXIT, NEW_EXIT_CODE),
    );
    let distinct_backing = registry.data_address(new).unwrap() != first_data;
    p.release(retired_extent);
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
            && distinct_backing
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
    let mut denied_spec = spec(0, MODE_DENIED_CREATE, EXIT_CODE);
    // Retain an in-kernel bound on this trusted probe's timer admissions.
    // Exhaustion is still a failed probe, never an accepted authority result.
    denied_spec.slice_limit = Some(SPIN_BUDGET);
    let denied_process = create(registry, p, denied_spec);
    registry.start(denied_process).unwrap();
    // This finite probe must reach the unsupported SVC. Expiring a verifier
    // deadline first tests elapsed host/QEMU time rather than authority. The
    // external QEMU runner still bounds a hung probe; scheduling deadlines and
    // workload slice budgets retain their own independent enforcement.
    registry.dispatch(None);
    let denied_reason = registry.completion(denied_process).unwrap().reason;
    let live = registry.live();
    let authority_passed = denied
        && invalid
        && denied_reason
            == Reason::Faulted {
                class: ESR_SVC64,
                address: 0,
            }
        && live == 1;
    #[cfg(feature = "diagnostics")]
    if !authority_passed {
        crate::diagnostics::status(
            "DEBUG",
            "process-authority",
            format_args!(
                "origin_denied={denied} invalid_image_rejected={invalid} completion={denied_reason:?} live={live}"
            ),
        );
    }
    report("process_authority_boundary", authority_passed);
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
    #[cfg(feature = "kernel-tests")]
    let measurement_start = cpu::ticks();
    #[cfg(feature = "kernel-tests")]
    let measurement_before = crate::asid::counters();
    let mut stress = true;
    let mut asid_reuse = true;
    let mut first_stress_failure = None;
    let mut cpu_isolation = [true; config::ACTIVE_CPUS];
    let mut backing_changed = [true; config::ACTIVE_CPUS];
    let mut previous_data = [0; config::ACTIVE_CPUS];
    let mut previous_asid = [0; config::ACTIVE_CPUS];
    let mut asid_reused = [false; config::ACTIVE_CPUS];
    let mut retired_extents: [Option<memory::Frame>; config::ACTIVE_CPUS] =
        core::array::from_fn(|_| None);
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
        if cycle != 0 {
            backing_changed[0] &= registry.data_address(left).unwrap() != previous_data[0];
            backing_changed[1] &= registry.data_address(right).unwrap() != previous_data[1];
            asid_reused[0] |= registry.asid(left).unwrap() == previous_asid[0];
            asid_reused[1] |= registry.asid(right).unwrap() == previous_asid[1];
        }
        for extent in &mut retired_extents {
            if let Some(frame) = extent.take() {
                p.release(frame);
            }
        }
        registry.start(left).unwrap();
        registry.start(right).unwrap();
        registry.dispatch(Some(VERIFY_TIMEOUT));
        let left_reason = registry.completion(left).unwrap().reason;
        let right_reason = registry.completion(right).unwrap().reason;
        let left_tag = tag(registry, left);
        let right_tag = tag(registry, right);
        let left_isolation = left_reason == Reason::Exited(code) && left_tag == code;
        let right_isolation = right_tag == code;
        if (!left_isolation || !right_isolation) && first_stress_failure.is_none() {
            first_stress_failure =
                Some((cycle, code, left_reason, right_reason, left_tag, right_tag));
        }
        cpu_isolation[0] &= left_isolation;
        cpu_isolation[1] &= right_isolation;
        let same_va_isolation = left_isolation && right_isolation;
        stress &= same_va_isolation;
        asid_reuse &= same_va_isolation;
        let expected = if cycle % 2 == 0 {
            Reason::Exited(code)
        } else {
            Reason::Faulted {
                class: ESR_DATA_ABORT_LOWER,
                address: memory::USER_GUARD,
            }
        };
        stress &= registry.completion(right).unwrap().reason == expected;
        previous_data = [
            registry.data_address(left).unwrap(),
            registry.data_address(right).unwrap(),
        ];
        previous_asid = [registry.asid(left).unwrap(), registry.asid(right).unwrap()];
        registry.reclaim(p, left).unwrap();
        registry.reclaim(p, right).unwrap();
        retired_extents[0] = Some(
            p.pin_extent_at(
                previous_data[0] - memory::USER_DATA_FRAME_OFFSET,
                memory::USER_SPACE_PAGES,
            )
            .expect("CPU0 retired extent witness"),
        );
        retired_extents[1] = Some(
            p.pin_extent_at(
                previous_data[1] - memory::USER_DATA_FRAME_OFFSET,
                memory::USER_SPACE_PAGES,
            )
            .expect("CPU1 retired extent witness"),
        );
        stress &= registry.live() == 0 && p.available() == before - 2 * memory::USER_SPACE_PAGES;
    }
    for extent in &mut retired_extents {
        if let Some(frame) = extent.take() {
            p.release(frame);
        }
    }
    let distinct_backing = backing_changed.into_iter().all(|changed| changed);
    #[cfg(feature = "kernel-tests")]
    let elapsed_ticks = cpu::ticks().wrapping_sub(measurement_start);
    #[cfg(feature = "kernel-tests")]
    let measurement_after = crate::asid::counters();
    #[cfg(feature = "kernel-tests")]
    let delta = core::array::from_fn::<_, { config::ACTIVE_CPUS }, _>(|owner| {
        let start = measurement_before[owner];
        let end = measurement_after[owner];
        crate::asid::Counters {
            switches: end.switches - start.switches,
            full_tlbi: end.full_tlbi - start.full_tlbi,
            asid_tlbi: end.asid_tlbi - start.asid_tlbi,
            reuses: end.reuses - start.reuses,
        }
    });
    #[cfg(feature = "kernel-tests")]
    let reuse_invalidation = if crate::asid::enabled() {
        delta
            .iter()
            .all(|count| count.asid_tlbi == STRESS_CYCLES as u64)
    } else {
        delta
            .iter()
            .all(|count| count.full_tlbi == count.switches && count.full_tlbi > 0)
    };
    #[cfg(not(feature = "kernel-tests"))]
    let reuse_invalidation = true;
    #[cfg(feature = "kernel-tests")]
    crate::event!(
        "{{\"event\":\"asid-measurement\",\"mode\":\"{}\",\"hardware_asid_bits\":{},\"reuse_invalidation\":{},\"elapsed_ticks\":{},\"same_va_isolation\":[{},{}],\"distinct_backing\":[{},{}],\"same_asid_reused\":[{},{}],\"cpus\":[{{\"cpu\":0,\"switches\":{},\"full_tlbi\":{},\"asid_tlbi\":{},\"reuses\":{}}},{{\"cpu\":1,\"switches\":{},\"full_tlbi\":{},\"asid_tlbi\":{},\"reuses\":{}}}]}}",
        if crate::asid::enabled() {
            "tagged"
        } else {
            "asid-zero-baseline"
        },
        cpu::asid_bits().unwrap_or(0),
        reuse_invalidation,
        elapsed_ticks,
        cpu_isolation[0],
        cpu_isolation[1],
        backing_changed[0],
        backing_changed[1],
        asid_reused[0],
        asid_reused[1],
        delta[0].switches,
        delta[0].full_tlbi,
        delta[0].asid_tlbi,
        delta[0].reuses,
        delta[1].switches,
        delta[1].full_tlbi,
        delta[1].asid_tlbi,
        delta[1].reuses,
    );
    if let Some((cycle, expected, left_reason, right_reason, left_tag, right_tag)) =
        first_stress_failure
    {
        crate::diagnostics::status(
            "FAIL",
            "process stress",
            format_args!(
                "cycle={cycle} expected={expected} cpu0_reason={left_reason:?} cpu1_reason={right_reason:?} cpu0_tag={left_tag} cpu1_tag={right_tag}"
            ),
        );
    }
    // The mutation must report its deterministic mechanism violation before
    // a stale translation can fail the dependent functional observation.
    report("asid_reuse_requires_invalidation", reuse_invalidation);
    report(
        "asid_reuse_same_va_both_cpus",
        asid_reuse && distinct_backing && asid_reused.into_iter().all(|reused| reused),
    );
    report("process_bounded_stress", stress);
    quantum_progress(p, registry, &mut report);
    blocking_progress(p, registry, &mut report);
    #[cfg(feature = "kernel-tests")]
    exercise_elf(p, registry, &mut report, before);
    report(
        "process_resource_reclamation",
        registry.live() == 0 && p.available() == before,
    );
}

#[cfg(feature = "kernel-tests")]
fn exercise_elf(
    p: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
    baseline_pages: usize,
) {
    let image = kernel_core::elf::parse(
        MINIMAL_ELF,
        config::USER_PAYLOAD_BASE,
        config::USER_PAYLOAD_BYTES,
        config::PAGE_BYTES,
    )
    .expect("checked-in minimal ELF fixture");
    let mut context = Context::ZERO;
    context.pc = image.entry as u64;
    context.sp = memory::USER_STACK_TOP as u64;

    let pages_before = p.available();
    let live_before = registry.live();
    let mut entry_alignment = true;
    for delta in [1usize, 2, 3, 15] {
        let mut damaged = alloc::vec::Vec::from(MINIMAL_ELF);
        let entry = config::USER_PAYLOAD_BASE + delta;
        damaged[24..32].copy_from_slice(&(entry as u64).to_le_bytes());
        entry_alignment &= kernel_core::elf::parse(
            &damaged,
            config::USER_PAYLOAD_BASE,
            config::USER_PAYLOAD_BYTES,
            config::PAGE_BYTES,
        )
        .is_err_and(|error| error == "AArch64 ELF entry is not instruction-aligned");
    }
    report("elf_entry_alignment", entry_alignment);
    let mut rejected = true;
    for (offset, value) in [
        (68usize, 7u64),
        (80, (config::USER_PAYLOAD_BASE - config::PAGE_BYTES) as u64),
        (24, (config::USER_PAYLOAD_BASE + config::PAGE_BYTES) as u64),
        (24, (config::USER_PAYLOAD_BASE + 1) as u64),
        (24, (config::USER_PAYLOAD_BASE + 2) as u64),
        (24, (config::USER_PAYLOAD_BASE + 3) as u64),
        (24, (config::USER_PAYLOAD_BASE + 4) as u64),
        (24, (config::USER_PAYLOAD_BASE + 15) as u64),
        (96, u64::MAX),
    ] {
        let mut damaged = alloc::vec::Vec::from(MINIMAL_ELF);
        damaged[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        // Match e_entry in Spec so a mismatch cannot mask profile rejection.
        let requested_entry = if offset == 24 {
            value as usize
        } else {
            image.entry
        };
        let mut requested_context = context;
        requested_context.pc = requested_entry as u64;
        let result = registry.create(
            p,
            Origin::Bootstrap,
            Spec {
                image: &damaged,
                image_format: process::ImageFormat::Elf64Aarch64,
                limits: limits(),
                context: requested_context,
                owner: 0,
                entry: requested_entry,
                slice_limit: Some(ELF_MAX_SLICES),
            },
            None,
        );
        rejected &= result.is_err_and(|failure| {
            failure.error == Error::InvalidImage && failure.completion.is_none()
        });
        rejected &= p.available() == pages_before && registry.live() == live_before;
    }
    report("elf_invalid_images_are_transactional", rejected);

    let mut injection_rollback = true;
    for step in kernel_core::process::CREATION_STEPS {
        let available = p.available();
        let live = registry.live();
        let result = registry.create(
            p,
            Origin::Bootstrap,
            Spec {
                image: MINIMAL_ELF,
                image_format: process::ImageFormat::Elf64Aarch64,
                limits: limits(),
                context,
                owner: 0,
                entry: image.entry,
                slice_limit: Some(ELF_MAX_SLICES),
            },
            Some(step),
        );
        injection_rollback &= result.is_err_and(|failure| {
            failure.error == Error::Allocation
                && failure.completion.is_some_and(|completion| {
                    completion.reason == Reason::CreationFailed(step)
                        && registry.state(completion.id) == Err(Error::Stale)
                })
        });
        injection_rollback &= p.available() == available && registry.live() == live;
    }
    report("elf_creation_failure_is_transactional", injection_rollback);

    let first = registry
        .create(
            p,
            Origin::Bootstrap,
            Spec {
                image: MINIMAL_ELF,
                image_format: process::ImageFormat::Elf64Aarch64,
                limits: limits(),
                context,
                owner: percpu::BOOT_CPU,
                entry: image.entry,
                slice_limit: Some(ELF_MAX_SLICES),
            },
            None,
        )
        .unwrap_or_else(|failure| process::reject(failure.error));
    let second = registry
        .create(
            p,
            Origin::Bootstrap,
            Spec {
                image: MINIMAL_ELF,
                image_format: process::ImageFormat::Elf64Aarch64,
                limits: limits(),
                context,
                owner: percpu::SECONDARY_CPU,
                entry: image.entry,
                slice_limit: Some(ELF_MAX_SLICES),
            },
            None,
        )
        .unwrap_or_else(|failure| process::reject(failure.error));
    registry.start(first).unwrap();
    registry.start(second).unwrap();
    registry.dispatch(Some(VERIFY_TIMEOUT));
    let exited = registry
        .completion(first)
        .is_ok_and(|done| done.reason == Reason::Exited(ELF_EXIT_CODE))
        && registry
            .completion(second)
            .is_ok_and(|done| done.reason == Reason::Exited(ELF_EXIT_CODE));
    let image_address = registry.image_address(first).unwrap();
    // SAFETY: image is retained after terminal completion, both CPUs detached, and the
    // verifier reads only the zero-filled tail of the private executable page.
    let zero_tail = unsafe {
        core::slice::from_raw_parts((image_address + 12) as *const u8, config::PAGE_BYTES - 12)
            .iter()
            .all(|byte| *byte == 0)
    };
    let isolated = registry.data_address(first).unwrap() != registry.data_address(second).unwrap();
    registry.reclaim(p, first).unwrap();
    registry.reclaim(p, second).unwrap();
    report("elf_executes_in_isolated_el0_spaces", exited && isolated);
    report("elf_bss_and_page_padding_are_zero", zero_tail);
    report(
        "elf_reclaims_all_frames",
        p.available() == baseline_pages && registry.live() == 0,
    );
}

fn quantum_progress(
    p: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let before = p.available();
    let ids: [ProcessId; config::ACTIVE_CPUS * 2] = core::array::from_fn(|index| {
        let spinning = index % 2 == 0;
        let mut definition = spec(
            index / 2,
            if spinning { MODE_SPIN } else { MODE_EXIT },
            EXIT_CODE + index as u64,
        );
        if spinning {
            definition.slice_limit = Some(SPIN_BUDGET);
            definition.context.gpr[19] = EXIT_CODE;
            definition.context.simd = core::array::from_fn(|register| {
                [EXIT_CODE + register as u64, !(EXIT_CODE + register as u64)]
            });
            definition.context.fpcr = 1 << 22;
            definition.context.fpsr = 1;
            definition.context.tpidr = EXIT_CODE + index as u64;
        }
        let id = create(registry, p, definition);
        registry.start(id).unwrap();
        id
    });
    let first = registry.step();
    let mut passed = first.owners_released;
    passed &= quantum_context(&first, &ids, registry);
    let mut counters = [0_u64; config::ACTIVE_CPUS];
    let mut el0_residency = [0_u64; config::ACTIVE_CPUS];
    let mut first_el0_residency = [0_u64; config::ACTIVE_CPUS];
    let mut el0_generation_valid = true;
    let mut el0_monotonic = true;
    for (owner, counter) in counters.iter_mut().enumerate() {
        let id = ids[owner * 2];
        let task = &first.tasks[id.slot()];
        el0_residency[owner] = task.el0_residency_ticks;
        first_el0_residency[owner] = task.el0_residency_ticks;
        el0_generation_valid &= task.process_generation == id.generation();
        passed &= registry.state(id) == Ok(State::Admitted)
            && registry.completion(id) == Err(Error::Transition)
            && registry.reclaim(p, id) == Err(Error::Transition);
        assert!(crate::scheduler::detached(id));
        // SAFETY: INV-USER-RETIRE: step restored native roots and all writers are
        // quiescent; the registry still retains the private space across steps.
        *counter = unsafe {
            core::ptr::read_volatile((registry.data_address(id).unwrap() + 8) as *const u64)
        };
        passed &= *counter > 0;
    }
    let mut peer_progress = [false; config::ACTIVE_CPUS];
    for _ in 0..SPIN_BUDGET * 3 {
        let result = registry.step();
        passed &= result.owners_released;
        passed &= quantum_context(&result, &ids, registry);
        for owner in 0..config::ACTIVE_CPUS {
            let spinner = ids[owner * 2];
            let peer = ids[owner * 2 + 1];
            let task = &result.tasks[spinner.slot()];
            if task.process_generation == spinner.generation() {
                el0_monotonic &= task.el0_residency_ticks >= el0_residency[owner];
                el0_residency[owner] = task.el0_residency_ticks;
            }
            if registry.state(spinner) == Ok(State::Admitted) && registry.completion(peer).is_ok() {
                peer_progress[owner] = true;
            }
        }
        if ids.iter().all(|id| registry.completion(*id).is_ok()) {
            break;
        }
    }
    for (index, id) in ids.into_iter().enumerate() {
        let expected = if index % 2 == 0 {
            Reason::BudgetExpired
        } else {
            Reason::Exited(EXIT_CODE + index as u64)
        };
        passed &= registry.completion(id).is_ok_and(|c| c.reason == expected);
        passed &= tag(registry, id) == EXIT_CODE + index as u64;
        if index % 2 == 0 {
            // SAFETY: INV-USER-RETIRE: terminal retained space, all CPU writers
            // quiescent; this read ends before reclamation below.
            let counter = unsafe {
                core::ptr::read_volatile((registry.data_address(id).unwrap() + 8) as *const u64)
            };
            passed &= counter > counters[index / 2];
        }
        registry.reclaim(p, id).unwrap();
    }
    report(
        "process_el0_residency_initial",
        el0_generation_valid && first_el0_residency.iter().all(|ticks| *ticks > 0),
    );
    report("process_el0_residency_monotonic", el0_monotonic);
    report(
        "process_el0_residency_accumulates",
        el0_residency
            .iter()
            .zip(first_el0_residency)
            .all(|(final_ticks, initial_ticks)| *final_ticks > initial_ticks),
    );
    report(
        "process_quantum_return_and_peer_progress",
        passed
            && peer_progress.into_iter().all(|progress| progress)
            && registry.live() == 0
            && p.available() == before,
    );
}

fn quantum_context(
    result: &crate::scheduler::Completed,
    ids: &[ProcessId],
    registry: &Registry,
) -> bool {
    ids.iter().step_by(2).all(|id| {
        let task = &result.tasks[id.slot()];
        // CPUs may finish in different rounds. A previously completed process
        // is intentionally absent from subsequent admissions and snapshots.
        if task.process_generation == 0 {
            return registry.completion(*id).is_ok();
        }
        let context = &task.context;
        context.gpr[19] == EXIT_CODE
            && context.simd.iter().enumerate().all(|(register, value)| {
                *value == [EXIT_CODE + register as u64, !(EXIT_CODE + register as u64)]
            })
            && context.fpcr == 1 << 22
            && context.fpsr == 1
            && context.tpidr == context.gpr[20]
            && context.sp == memory::USER_STACK_TOP as u64
    })
}

fn blocking_progress(
    p: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let before = p.available();
    let ids: [ProcessId; config::ACTIVE_CPUS] = core::array::from_fn(|owner| {
        let id = create(
            registry,
            p,
            spec(owner, MODE_WAIT, EXIT_CODE + owner as u64),
        );
        registry.start(id).unwrap();
        id
    });
    let mut passed = registry.signal(ids[0]) == Ok(true) && registry.signal(ids[0]) == Ok(false);
    for _ in 0..12 {
        advance(registry);
        if ids.iter().all(|id| registry.blocked(*id) == Ok(true)) {
            break;
        }
    }
    passed &= ids.iter().all(|id| registry.blocked(*id) == Ok(true));
    let idle = registry.step();
    passed &= idle.switches == 0 && idle.owners_released;
    for id in ids {
        passed &= registry.completion(id) == Err(Error::Transition)
            && registry.reclaim(p, id) == Err(Error::Transition);
        registry.signal(id).unwrap();
    }
    for _ in 0..12 {
        advance(registry);
        if registry.completion(ids[0]).is_ok() && registry.blocked(ids[1]) == Ok(true) {
            break;
        }
    }
    passed &= registry
        .completion(ids[0])
        .is_ok_and(|c| c.reason == Reason::Exited(EXIT_CODE))
        && registry.blocked(ids[1]) == Ok(true);
    registry.signal(ids[1]).unwrap();
    for _ in 0..12 {
        advance(registry);
        if registry.completion(ids[1]).is_ok() {
            break;
        }
    }
    for (owner, id) in ids.into_iter().enumerate() {
        passed &= registry
            .completion(id)
            .is_ok_and(|c| c.reason == Reason::Exited(EXIT_CODE + owner as u64));
        passed &= registry.signal(id) == Err(Error::Transition);
        registry.reclaim(p, id).unwrap();
    }
    let replacement = create(registry, p, spec(0, MODE_WAIT, EXIT_CODE));
    registry.start(replacement).unwrap();
    passed &= registry.signal(ids[0]) == Err(Error::Stale);
    for _ in 0..12 {
        advance(registry);
        if registry.blocked(replacement) == Ok(true) {
            break;
        }
    }
    passed &= registry.blocked(replacement) == Ok(true);
    // Two distinct notifications release the two waits, retaining the second
    // notification even though it arrives before the next registration.
    registry.signal(replacement).unwrap();
    registry.signal(replacement).unwrap();
    for _ in 0..12 {
        advance(registry);
        if registry.completion(replacement).is_ok() {
            break;
        }
    }
    passed &= registry.completion(replacement).is_ok();
    registry.reclaim(p, replacement).unwrap();
    report(
        "process_wait_block_and_wakeup",
        passed && registry.live() == 0 && p.available() == before,
    );
}

// Keep discarded large snapshots in one short-lived fixture frame, rather than
// reserving one debug-build stack temporary for every call site in the scenario.
fn advance(registry: &mut Registry) {
    registry.step();
}

fn limits() -> kernel_core::domain::Limits {
    kernel_core::domain::Limits {
        memory_pages: crate::memory::USER_SPACE_PAGES
            + crate::platform::config::USER_PAYLOAD_BYTES / crate::platform::config::PAGE_BYTES,
        handles: crate::handles::CAPACITY as u16,
        queue: 1,
        requests: 1,
        endpoints: 0,
    }
}
