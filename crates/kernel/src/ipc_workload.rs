//! Real EL0 peers use the production native.request/1 transport. Bootstrap only
//! installs explicit receiver/SEND bindings; no Rust helper performs their IPC.
use crate::{
    cpu::context::Context,
    memory,
    process::{ImageFormat, Origin, Reason, Registry, Spec},
};
use kernel_core::{domain::Limits, handles::Rights};
// SAFETY: INV-USER-IMAGE: immutable position-independent bounded EL0 image.
core::arch::global_asm!(include_str!("ipc_workload.S"));
// SAFETY: INV-USER-IMAGE: separate bounded immutable position-independent EL0 image.
core::arch::global_asm!(include_str!("ipc_death_workload.S"));
// SAFETY: INV-USER-IMAGE: bounded immutable position-independent authority fixture.
core::arch::global_asm!(include_str!("ipc_authority_workload.S"));
// SAFETY: INV-USER-IMAGE: bounded immutable position-independent payload stress image.
core::arch::global_asm!(include_str!("ipc_payload_workload.S"));
// SAFETY: INV-USER-IMAGE: bounded immutable position-independent queue fixture.
core::arch::global_asm!(include_str!("ipc_queue_workload.S"));
// SAFETY: INV-USER-IMAGE: bounded immutable concurrent-producer fixture.
core::arch::global_asm!(include_str!("ipc_multi_workload.S"));
// SAFETY: INV-USER-IMAGE: bounded immutable two-producer revoke-race fixture.
core::arch::global_asm!(include_str!("ipc_revoke_workload.S"));
unsafe extern "C" {
    static ipc_image_start: u8;
    static ipc_image_end: u8;
    static ipc_death_image_start: u8;
    static ipc_death_image_end: u8;
    static ipc_authority_image_start: u8;
    static ipc_authority_image_end: u8;
    static ipc_payload_image_start: u8;
    static ipc_payload_image_end: u8;
    static ipc_queue_image_start: u8;
    static ipc_queue_image_end: u8;
    static ipc_multi_image_start: u8;
    static ipc_multi_image_end: u8;
    static ipc_revoke_image_start: u8;
    static ipc_revoke_image_end: u8;
}
fn create_peer(
    registry: &mut Registry,
    physical: &mut memory::Physical,
    image: &[u8],
    owner: usize,
) -> kernel_core::process::ProcessId {
    let mut context = Context::ZERO;
    context.pc = memory::USER_CODE as u64;
    context.sp = memory::USER_STACK_TOP as u64;
    registry
        .create(
            physical,
            Origin::Bootstrap,
            Spec {
                image,
                image_format: ImageFormat::RawFixture,
                context,
                owner,
                entry: memory::USER_CODE,
                slice_limit: None,
                limits: Limits {
                    memory_pages: memory::USER_SPACE_PAGES,
                    handles: 4,
                    queue: 4,
                    requests: 8,
                    endpoints: 1,
                },
            },
            None,
        )
        .unwrap()
}
pub(crate) fn exercise(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    mut report: impl FnMut(&str, bool),
) {
    let start = &raw const ipc_image_start as usize;
    let end = &raw const ipc_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: checked permanent linked image extent, no mutation.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    for (name, client_cpu, service_cpu) in [
        ("ipc_el0_cpu0_to_cpu1", 0, 1),
        ("ipc_el0_cpu1_to_cpu0", 1, 0),
        ("ipc_el0_same_cpu0", 0, 0),
        ("ipc_el0_same_cpu1", 1, 1),
    ] {
        let mut passed = true;
        let mut previous_service: Option<(kernel_core::process::ProcessId, u64)> = None;
        for capacity in [1, 4] {
            let create = |registry: &mut Registry, physical: &mut memory::Physical, owner| {
                let mut context = Context::ZERO;
                context.pc = memory::USER_CODE as u64;
                context.sp = memory::USER_STACK_TOP as u64;
                registry
                    .create(
                        physical,
                        Origin::Bootstrap,
                        Spec {
                            image,
                            image_format: ImageFormat::RawFixture,
                            context,
                            owner,
                            entry: memory::USER_CODE,
                            slice_limit: None,
                            limits: Limits {
                                memory_pages: memory::USER_SPACE_PAGES,
                                handles: 2,
                                queue: 4,
                                requests: 8,
                                endpoints: 1,
                            },
                        },
                        None,
                    )
                    .unwrap()
            };
            let client = create(registry, physical, client_cpu);
            let service = create(registry, physical, service_cpu);
            let (endpoint, receiver) = registry.bootstrap_endpoint(service, capacity).unwrap();
            let sender = registry
                .bootstrap_sender(client, &endpoint, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, 0, sender);
            registry.ipc_input(service, 1, receiver);
            if let Some((previous, token)) = previous_service {
                assert_eq!(previous.slot(), service.slot());
                assert!(previous.generation() < service.generation() && token != 0);
                registry.ipc_previous_token_input(service, token);
            }
            registry.start(client).unwrap();
            registry.start(service).unwrap();
            let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
            previous_service = Some((service, completed.tasks[service.slot()].context.gpr[22]));
            // A condition may become ready before registration. Every actual
            // registration must have exactly one owner-local READY publication.
            for id in [client, service] {
                let task = &completed.tasks[id.slot()];
                passed &= task.ipc_blocks == task.ipc_wakes;
            }
            for id in [client, service] {
                let completion = registry.completion(id).unwrap();
                if completion.reason != Reason::Exited(1) {
                    crate::event!(
                        "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{},\"actual\":{},\"value\":{}}}",
                        id.slot(),
                        completed.tasks[id.slot()].state,
                        completed.tasks[id.slot()].context.gpr[0],
                        completed.tasks[id.slot()].fault_class,
                        completed.tasks[id.slot()].context.gpr[29],
                        completed.tasks[id.slot()].context.gpr[1]
                    );
                    passed = false;
                }
                registry.reclaim(physical, id).unwrap();
            }
            drop(endpoint);
            crate::ipc::reap();
            passed &= physical.available() == before && completed.owners_released;
            crate::event!(
                "{{\"event\":\"ipc\",\"status\":\"{}\",\"name\":\"{}\",\"capacity\":{},\"client_cpu\":{},\"service_cpu\":{},\"client_generation\":{},\"service_generation\":{},\"client_blocks\":{},\"client_wakes\":{},\"service_blocks\":{},\"service_wakes\":{}}}",
                if passed { "pass" } else { "fail" },
                name,
                capacity,
                client_cpu,
                service_cpu,
                client.generation(),
                service.generation(),
                completed.tasks[client.slot()].ipc_blocks,
                completed.tasks[client.slot()].ipc_wakes,
                completed.tasks[service.slot()].ipc_blocks,
                completed.tasks[service.slot()].ipc_wakes
            );
        }
        report(name, passed);
    }
    for (name, client_role, service_role, outcome) in [
        ("ipc_deadline_before_effect", 2, 3, 6),
        ("ipc_deadline_after_commit", 4, 5, 5),
    ] {
        let mut passed = true;
        for (client_cpu, service_cpu) in [(0, 1), (1, 0)] {
            let create = |registry: &mut Registry, physical: &mut memory::Physical, owner| {
                let mut context = Context::ZERO;
                context.pc = memory::USER_CODE as u64;
                context.sp = memory::USER_STACK_TOP as u64;
                registry
                    .create(
                        physical,
                        Origin::Bootstrap,
                        Spec {
                            image,
                            image_format: ImageFormat::RawFixture,
                            context,
                            owner,
                            entry: memory::USER_CODE,
                            slice_limit: None,
                            limits: Limits {
                                memory_pages: memory::USER_SPACE_PAGES,
                                handles: 2,
                                queue: 4,
                                requests: 8,
                                endpoints: 1,
                            },
                        },
                        None,
                    )
                    .unwrap()
            };
            let client = create(registry, physical, client_cpu);
            let service = create(registry, physical, service_cpu);
            let domains = [
                registry.domain_reference(client),
                registry.domain_reference(service),
            ];
            let (endpoint, receiver) = registry.bootstrap_endpoint(service, 1).unwrap();
            let sender = registry
                .bootstrap_sender(client, &endpoint, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, client_role, sender);
            registry.ipc_input(service, service_role, receiver);
            registry.start(client).unwrap();
            registry.start(service).unwrap();
            let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
            passed &= completed.ipc_all_blocked > 0;
            for id in [client, service] {
                let task = &completed.tasks[id.slot()];
                passed &= registry.completion(id).unwrap().reason == Reason::Exited(1)
                    && task.ipc_blocks > 0
                    && task.ipc_blocks == task.ipc_wakes;
                if registry.completion(id).unwrap().reason != Reason::Exited(1) {
                    crate::event!(
                        "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{}}}",
                        id.slot(),
                        task.state,
                        task.context.gpr[0],
                        task.fault_class
                    );
                }
                registry.reclaim(physical, id).unwrap();
            }
            drop(endpoint);
            crate::ipc::reap();
            passed &= completed.owners_released
                && physical.available() == before
                && domains
                    .iter()
                    .all(|domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0)
                && crate::ipc::deferred::mailboxes_quiescent();
            crate::event!(
                "{{\"event\":\"ipc-deadline\",\"status\":\"{}\",\"name\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"outcome\":{},\"all_blocked\":{},\"client_blocks\":{},\"client_wakes\":{},\"service_blocks\":{},\"service_wakes\":{},\"reclaimed\":true}}",
                if passed { "pass" } else { "fail" },
                name,
                client_cpu,
                service_cpu,
                outcome,
                completed.ipc_all_blocked,
                completed.tasks[client.slot()].ipc_blocks,
                completed.tasks[client.slot()].ipc_wakes,
                completed.tasks[service.slot()].ipc_blocks,
                completed.tasks[service.slot()].ipc_wakes
            );
        }
        report(name, passed);
    }
    for (name, client_role, service_role, outcome) in [
        ("ipc_service_death_queued", 6, 7, 4),
        ("ipc_service_death_delivered", 8, 9, 4),
        ("ipc_service_death_committed", 10, 11, 5),
        ("ipc_receive_copy_failure_retains_queue", 0, 13, 0),
        ("ipc_collect_copy_failure_retains_result", 14, 1, 0),
        ("ipc_cancel_before_commit", 16, 17, 4),
        ("ipc_cancel_after_commit", 18, 19, 5),
    ] {
        let mut passed = true;
        for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
            let client = create_peer(registry, physical, image, client_cpu);
            let service = create_peer(registry, physical, image, service_cpu);
            let control = if client_role >= 16 {
                let (control, control_receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
                let control_sender = registry
                    .bootstrap_sender(service, &control, Rights::SEND)
                    .unwrap();
                registry.ipc_control_input(client, control_receiver);
                registry.ipc_control_input(service, control_sender);
                Some(control)
            } else {
                None
            };
            let domains = [
                registry.domain_reference(client),
                registry.domain_reference(service),
            ];
            let (endpoint, receiver) = registry.bootstrap_endpoint(service, 1).unwrap();
            let sender = registry
                .bootstrap_sender(client, &endpoint, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, client_role, sender);
            registry.ipc_input(service, service_role, receiver);
            registry.start(client).unwrap();
            registry.start(service).unwrap();
            let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
            for id in [client, service] {
                let task = &completed.tasks[id.slot()];
                passed &= registry.completion(id).unwrap().reason == Reason::Exited(1)
                    && task.ipc_blocks == task.ipc_wakes;
                if registry.completion(id).unwrap().reason != Reason::Exited(1) {
                    crate::event!(
                        "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{}}}",
                        id.slot(),
                        task.state,
                        task.context.gpr[0],
                        task.fault_class
                    );
                }
                registry.reclaim(physical, id).unwrap();
            }
            drop(endpoint);
            crate::ipc::reap();
            drop(control);
            crate::ipc::reap();
            let reclaimed = completed.owners_released
                && physical.available() == before
                && domains
                    .iter()
                    .all(|domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0)
                && crate::ipc::deferred::mailboxes_quiescent();
            passed &= reclaimed;
            crate::event!(
                "{{\"event\":\"ipc-lifetime\",\"status\":\"{}\",\"name\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"outcome\":{},\"client_blocks\":{},\"client_wakes\":{},\"service_blocks\":{},\"service_wakes\":{},\"reclaimed\":{}}}",
                if passed { "pass" } else { "fail" },
                name,
                client_cpu,
                service_cpu,
                outcome,
                completed.tasks[client.slot()].ipc_blocks,
                completed.tasks[client.slot()].ipc_wakes,
                completed.tasks[service.slot()].ipc_blocks,
                completed.tasks[service.slot()].ipc_wakes,
                reclaimed
            );
        }
        report(name, passed);
    }
    requester_death(physical, registry, &mut report);
    authority(physical, registry, &mut report);
    payload_stress(physical, registry, &mut report);
    queue_fifo(physical, registry, &mut report);
    multiple_producers(physical, registry, &mut report);
    revoke_race(physical, registry, &mut report);
}
fn revoke_race(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let start = &raw const ipc_revoke_image_start as usize;
    let end = &raw const ipc_revoke_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact checked permanent read-only linked image.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    let mut passed = true;
    for (first_cpu, second_cpu, service_cpu) in [(0, 1, 0), (0, 1, 1), (0, 0, 0), (1, 1, 1)] {
        let mut ids = [None; 3];
        for (role, owner) in [first_cpu, second_cpu, service_cpu].into_iter().enumerate() {
            let mut context = Context::ZERO;
            context.pc = memory::USER_CODE as u64;
            context.sp = memory::USER_STACK_TOP as u64;
            ids[role] = Some(
                registry
                    .create(
                        physical,
                        Origin::Bootstrap,
                        Spec {
                            image,
                            image_format: ImageFormat::RawFixture,
                            context,
                            owner,
                            entry: memory::USER_CODE,
                            slice_limit: None,
                            limits: Limits {
                                memory_pages: memory::USER_SPACE_PAGES,
                                handles: 4,
                                queue: 8,
                                requests: 8,
                                endpoints: 2,
                            },
                        },
                        None,
                    )
                    .unwrap(),
            );
        }
        let ids = ids.map(Option::unwrap);
        let service = ids[2];
        let domains = ids.map(|id| registry.domain_reference(id));
        let (work, receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
        let (control, control_receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
        for (role, client) in ids[..2].iter().copied().enumerate() {
            let work_sender = registry
                .bootstrap_sender(
                    client,
                    &work,
                    if role == 0 {
                        Rights::ISSUER
                    } else {
                        Rights::SEND
                    },
                )
                .unwrap();
            let control_sender = registry
                .bootstrap_sender(client, &control, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, role as u64, work_sender);
            registry.ipc_control_input(client, control_sender);
        }
        registry.ipc_input(service, 2, receiver);
        registry.ipc_control_input(service, control_receiver);
        for id in ids {
            registry.start(id).unwrap();
        }
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        for id in ids {
            let task = &completed.tasks[id.slot()];
            let exit = registry.completion(id).unwrap().reason == Reason::Exited(1);
            passed &= exit && task.ipc_blocks == task.ipc_wakes;
            if !exit {
                crate::event!(
                    "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"code\":{},\"actual\":{},\"value\":{}}}",
                    id.slot(),
                    task.context.gpr[0],
                    task.context.gpr[29],
                    task.context.gpr[1]
                );
            }
            registry.reclaim(physical, id).unwrap();
        }
        drop(work);
        drop(control);
        crate::ipc::reap();
        let reclaimed = completed.owners_released
            && physical.available() == before
            && domains
                .iter()
                .all(|d| d.usage() == (0, 0, 0, 0) && d.endpoint_usage() == 0)
            && crate::ipc::deferred::mailboxes_quiescent();
        passed &= reclaimed;
        crate::event!(
            "{{\"event\":\"ipc-revoke-race\",\"status\":\"{}\",\"first_cpu\":{},\"second_cpu\":{},\"service_cpu\":{},\"attempts_per_client\":16,\"issuer_post_revoke_denials\":14,\"accepted_completed\":{},\"reclaimed\":{}}}",
            if passed { "pass" } else { "fail" },
            first_cpu,
            second_cpu,
            service_cpu,
            completed.tasks[service.slot()].context.gpr[16],
            reclaimed
        );
    }
    report("ipc_concurrent_revoke_admission_retains_accepted", passed);
}
fn multiple_producers(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let start = &raw const ipc_multi_image_start as usize;
    let end = &raw const ipc_multi_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: checked exact immutable linked instruction range.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    let mut passed = true;
    for (first_cpu, second_cpu, service_cpu) in [(0, 0, 0), (1, 1, 1), (0, 1, 0), (0, 1, 1)] {
        let first = create_peer(registry, physical, image, first_cpu);
        let second = create_peer(registry, physical, image, second_cpu);
        let service = create_peer(registry, physical, image, service_cpu);
        let ids = [first, second, service];
        let domains = ids.map(|id| registry.domain_reference(id));
        let (endpoint, receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
        for (role, client) in [first, second].into_iter().enumerate() {
            let sender = registry
                .bootstrap_sender(client, &endpoint, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, role as u64, sender);
        }
        registry.ipc_input(service, 2, receiver);
        for id in ids {
            registry.start(id).unwrap();
        }
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        for id in ids {
            let task = &completed.tasks[id.slot()];
            passed &= registry.completion(id).unwrap().reason == Reason::Exited(1)
                && task.ipc_blocks == task.ipc_wakes;
            registry.reclaim(physical, id).unwrap();
        }
        drop(endpoint);
        crate::ipc::reap();
        let reclaimed = completed.owners_released
            && physical.available() == before
            && domains
                .iter()
                .all(|d| d.usage() == (0, 0, 0, 0) && d.endpoint_usage() == 0)
            && crate::ipc::deferred::mailboxes_quiescent();
        passed &= reclaimed;
        crate::event!(
            "{{\"event\":\"ipc-multiple-producers\",\"status\":\"{}\",\"first_cpu\":{},\"second_cpu\":{},\"service_cpu\":{},\"capacity\":4,\"requests\":32,\"fifo\":true,\"reclaimed\":{}}}",
            if passed { "pass" } else { "fail" },
            first_cpu,
            second_cpu,
            service_cpu,
            reclaimed
        );
    }
    report("ipc_concurrent_producers_fifo_and_reclamation", passed);
}
fn queue_fifo(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let start = &raw const ipc_queue_image_start as usize;
    let end = &raw const ipc_queue_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact bounded linked immutable instruction extent.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    let mut passed = true;
    let mut cancel_passed = true;
    let mut both_death_passed = true;
    let mut shutdown_passed = true;
    for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
        for (capacity, mode) in [(1, 0), (4, 0), (4, 2), (4, 4), (4, 6)] {
            let cancel = mode == 2;
            let mut case_passed = true;
            let client = create_peer(registry, physical, image, client_cpu);
            let service = create_peer(registry, physical, image, service_cpu);
            let domains = [
                registry.domain_reference(client),
                registry.domain_reference(service),
            ];
            let (work, receiver) = registry.bootstrap_endpoint(service, capacity).unwrap();
            let (control, control_receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
            let sender = registry
                .bootstrap_sender(client, &work, Rights::SEND)
                .unwrap();
            let control_sender = registry
                .bootstrap_sender(service, &control, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, mode, sender);
            registry.ipc_control_input(client, control_receiver);
            registry.ipc_queue_input(client, capacity);
            registry.ipc_input(service, mode + 1, receiver);
            registry.ipc_control_input(service, control_sender);
            registry.ipc_queue_input(service, capacity);
            registry.start(client).unwrap();
            registry.start(service).unwrap();
            let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
            if mode == 6 {
                case_passed &= completed.tasks[client.slot()].ipc_terminal_blocks > 0;
            }
            for id in [client, service] {
                let task = &completed.tasks[id.slot()];
                let exit = registry.completion(id).unwrap().reason == Reason::Exited(1);
                case_passed &= exit && task.ipc_blocks == task.ipc_wakes;
                if !exit {
                    crate::event!(
                        "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{},\"actual\":{},\"value\":{}}}",
                        id.slot(),
                        task.state,
                        task.context.gpr[0],
                        task.fault_class,
                        task.context.gpr[29],
                        task.context.gpr[1]
                    );
                }
                registry.reclaim(physical, id).unwrap();
            }
            drop(work);
            drop(control);
            crate::ipc::reap();
            let reclaimed = completed.owners_released
                && physical.available() == before
                && domains
                    .iter()
                    .all(|domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0)
                && crate::ipc::deferred::mailboxes_quiescent();
            case_passed &= reclaimed;
            match mode {
                0 => passed &= case_passed,
                2 => cancel_passed &= case_passed,
                4 => both_death_passed &= case_passed,
                6 => shutdown_passed &= case_passed,
                _ => unreachable!("bounded queue fixture mode"),
            }
            crate::event!(
                "{{\"event\":\"{}\",\"status\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"capacity\":{},\"delivered\":{},\"full_rejected\":true,\"fifo\":true,\"reclaimed\":{},\"client_blocks\":{},\"client_terminal_blocks\":{},\"client_wakes\":{},\"service_blocks\":{},\"service_wakes\":{}}}",
                match mode {
                    0 => "ipc-queue",
                    2 => "ipc-queued-cancel",
                    4 => "ipc-both-death",
                    6 => "ipc-shutdown-load",
                    _ => unreachable!(),
                },
                if case_passed { "pass" } else { "fail" },
                client_cpu,
                service_cpu,
                capacity,
                if cancel {
                    2
                } else if mode >= 4 {
                    0
                } else {
                    capacity
                },
                reclaimed,
                completed.tasks[client.slot()].ipc_blocks,
                completed.tasks[client.slot()].ipc_terminal_blocks,
                completed.tasks[client.slot()].ipc_wakes,
                completed.tasks[service.slot()].ipc_blocks,
                completed.tasks[service.slot()].ipc_wakes
            );
        }
    }
    report("ipc_queue_full_fifo_and_reclamation", passed);
    report("ipc_queued_head_nonhead_cancel_fifo", cancel_passed);
    report("ipc_both_peers_die_with_accepted_work", both_death_passed);
    report("ipc_shutdown_queued_and_blocked_requester", shutdown_passed);
    service_empty_death(physical, registry, report, image);
}
fn service_empty_death(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
    image: &[u8],
) {
    let before = physical.available();
    let mut passed = true;
    for cpu in [0, 1] {
        let service = create_peer(registry, physical, image, cpu);
        let domain = registry.domain_reference(service);
        let (endpoint, receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
        registry.ipc_input(service, 9, receiver);
        registry.start(service).unwrap();
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        passed &= registry.completion(service).unwrap().reason == Reason::Exited(1)
            && completed.owners_released;
        registry.reclaim(physical, service).unwrap();
        drop(endpoint);
        crate::ipc::reap();
        let reclaimed = physical.available() == before
            && domain.usage() == (0, 0, 0, 0)
            && domain.endpoint_usage() == 0
            && crate::ipc::deferred::mailboxes_quiescent();
        passed &= reclaimed;
        crate::event!(
            "{{\"event\":\"ipc-empty-service-death\",\"status\":\"{}\",\"service_cpu\":{},\"reclaimed\":{}}}",
            if passed { "pass" } else { "fail" },
            cpu,
            reclaimed
        );
    }
    report("ipc_empty_service_death_reclamation", passed);
}
fn payload_stress(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let start = &raw const ipc_payload_image_start as usize;
    let end = &raw const ipc_payload_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact immutable linked image with checked byte extent.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    let mut passed = true;
    for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
        for capacity in [1, 4] {
            let client = create_peer(registry, physical, image, client_cpu);
            let service = create_peer(registry, physical, image, service_cpu);
            let domains = [
                registry.domain_reference(client),
                registry.domain_reference(service),
            ];
            let (work, receiver) = registry.bootstrap_endpoint(service, capacity).unwrap();
            let (control, control_receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
            let sender = registry
                .bootstrap_sender(client, &work, Rights::SEND)
                .unwrap();
            let control_sender = registry
                .bootstrap_sender(service, &control, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, 0, sender);
            registry.ipc_control_input(client, control_receiver);
            registry.ipc_input(service, 1, receiver);
            registry.ipc_control_input(service, control_sender);
            registry.start(client).unwrap();
            registry.start(service).unwrap();
            let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
            for id in [client, service] {
                let task = &completed.tasks[id.slot()];
                let exit = registry.completion(id).unwrap().reason == Reason::Exited(1);
                passed &= exit && task.ipc_blocks == task.ipc_wakes;
                if !exit {
                    crate::event!(
                        "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{},\"actual\":{},\"value\":{}}}",
                        id.slot(),
                        task.state,
                        task.context.gpr[0],
                        task.fault_class,
                        task.context.gpr[29],
                        task.context.gpr[1]
                    );
                }
                registry.reclaim(physical, id).unwrap();
            }
            drop(work);
            drop(control);
            crate::ipc::reap();
            let reclaimed = completed.owners_released
                && physical.available() == before
                && domains
                    .iter()
                    .all(|domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0)
                && crate::ipc::deferred::mailboxes_quiescent();
            passed &= reclaimed;
            crate::event!(
                "{{\"event\":\"ipc-payload-stress\",\"status\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"capacity\":{},\"requests\":24,\"payload_sizes\":[0,1,8,64,255,256],\"reclaimed\":{}}}",
                if passed { "pass" } else { "fail" },
                client_cpu,
                service_cpu,
                capacity,
                reclaimed
            );
            if !passed {
                report("ipc_payload_snapshot_result_id_and_stress", false);
                return;
            }
        }
    }
    report("ipc_payload_snapshot_result_id_and_stress", passed);
    terminal_result_death(physical, registry, report, image);
}
fn terminal_result_death(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
    image: &[u8],
) {
    let before = physical.available();
    let mut passed = true;
    for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
        let client = create_peer(registry, physical, image, client_cpu);
        let service = create_peer(registry, physical, image, service_cpu);
        let domains = [
            registry.domain_reference(client),
            registry.domain_reference(service),
        ];
        let (work, receiver) = registry.bootstrap_endpoint(service, 1).unwrap();
        let (control, control_receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
        let sender = registry
            .bootstrap_sender(client, &work, Rights::SEND)
            .unwrap();
        let control_sender = registry
            .bootstrap_sender(service, &control, Rights::SEND)
            .unwrap();
        registry.ipc_input(client, 2, sender);
        registry.ipc_control_input(client, control_receiver);
        registry.ipc_input(service, 3, receiver);
        registry.ipc_control_input(service, control_sender);
        registry.start(client).unwrap();
        registry.start(service).unwrap();
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        for id in [client, service] {
            let task = &completed.tasks[id.slot()];
            passed &= registry.completion(id).unwrap().reason == Reason::Exited(1)
                && task.ipc_blocks == task.ipc_wakes;
            registry.reclaim(physical, id).unwrap();
        }
        drop(work);
        drop(control);
        crate::ipc::reap();
        let reclaimed = completed.owners_released
            && physical.available() == before
            && domains
                .iter()
                .all(|d| d.usage() == (0, 0, 0, 0) && d.endpoint_usage() == 0)
            && crate::ipc::deferred::mailboxes_quiescent();
        passed &= reclaimed;
        crate::event!(
            "{{\"event\":\"ipc-terminal-death\",\"status\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"completed_unconsumed\":true,\"reclaimed\":{}}}",
            if passed { "pass" } else { "fail" },
            client_cpu,
            service_cpu,
            reclaimed
        );
    }
    report("ipc_unconsumed_terminal_domain_teardown", passed);
}
fn authority(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let start = &raw const ipc_authority_image_start as usize;
    let end = &raw const ipc_authority_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact bounded immutable linked instruction extent.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    let mut passed = true;
    for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
        let client = create_peer(registry, physical, image, client_cpu);
        let service = create_peer(registry, physical, image, service_cpu);
        let domains = [
            registry.domain_reference(client),
            registry.domain_reference(service),
        ];
        let (work, receiver) = registry.bootstrap_endpoint(service, 1).unwrap();
        let (control, control_receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
        let sender = registry
            .bootstrap_sender(client, &work, Rights::ISSUER)
            .unwrap();
        let missing = registry
            .bootstrap_sender(client, &work, Rights::NONE)
            .unwrap();
        let event = registry
            .bootstrap_grant(
                client,
                service,
                kernel_core::wait::SharedEvent::try_new().unwrap(),
                Rights::SEND,
            )
            .unwrap();
        let control_sender = registry
            .bootstrap_sender(service, &control, Rights::SEND)
            .unwrap();
        registry.ipc_input(client, 0, sender);
        registry.ipc_authority_input(client, missing, event, control_receiver);
        registry.ipc_input(service, 1, receiver);
        registry.ipc_control_input(service, control_sender);
        registry.start(client).unwrap();
        registry.start(service).unwrap();
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        for id in [client, service] {
            let task = &completed.tasks[id.slot()];
            let exit = registry.completion(id).unwrap().reason == Reason::Exited(1);
            passed &= exit && task.ipc_blocks == task.ipc_wakes;
            if !exit {
                crate::event!(
                    "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{}}}",
                    id.slot(),
                    task.state,
                    task.context.gpr[0],
                    task.fault_class
                );
            }
            registry.reclaim(physical, id).unwrap();
        }
        drop(work);
        drop(control);
        crate::ipc::reap();
        let reclaimed = completed.owners_released
            && physical.available() == before
            && domains
                .iter()
                .all(|domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0)
            && crate::ipc::deferred::mailboxes_quiescent();
        passed &= reclaimed;
        crate::event!(
            "{{\"event\":\"ipc-authority\",\"status\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"denied_probes\":13,\"accepted_after_revoke_close\":true,\"reclaimed\":{}}}",
            if passed { "pass" } else { "fail" },
            client_cpu,
            service_cpu,
            reclaimed
        );
    }
    report("ipc_authority_denial_revoke_and_retention", passed);
    request_quota_rollback(physical, registry, report, image);
}
fn request_quota_rollback(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
    image: &[u8],
) {
    let before = physical.available();
    let mut passed = true;
    for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
        let mut context = Context::ZERO;
        context.pc = memory::USER_CODE as u64;
        context.sp = memory::USER_STACK_TOP as u64;
        let client = registry
            .create(
                physical,
                Origin::Bootstrap,
                Spec {
                    image,
                    image_format: ImageFormat::RawFixture,
                    context,
                    owner: client_cpu,
                    entry: memory::USER_CODE,
                    slice_limit: None,
                    limits: Limits {
                        memory_pages: memory::USER_SPACE_PAGES,
                        handles: 1,
                        queue: 0,
                        requests: 0,
                        endpoints: 0,
                    },
                },
                None,
            )
            .unwrap();
        let service = create_peer(registry, physical, image, service_cpu);
        let domains = [
            registry.domain_reference(client),
            registry.domain_reference(service),
        ];
        let (endpoint, receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
        let sender = registry
            .bootstrap_sender(client, &endpoint, Rights::SEND)
            .unwrap();
        registry.ipc_input(client, 2, sender);
        registry.ipc_input(service, 9, receiver);
        // A bound prepared service keeps admission authority live while the
        // requester exercises its exact zero quota; only then launch its exit.
        registry.start(client).unwrap();
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        passed &= registry.completion(client).unwrap().reason == Reason::Exited(1)
            && completed.owners_released;
        registry.reclaim(physical, client).unwrap();
        passed &= domains[0].usage() == (0, 0, 0, 0)
            && domains[1].usage().2 == 0
            && domains[1].usage().3 == 0;
        registry.start(service).unwrap();
        let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
        passed &= registry.completion(service).unwrap().reason == Reason::Exited(1)
            && completed.owners_released;
        registry.reclaim(physical, service).unwrap();
        drop(endpoint);
        crate::ipc::reap();
        let reclaimed = physical.available() == before
            && domains
                .iter()
                .all(|d| d.usage() == (0, 0, 0, 0) && d.endpoint_usage() == 0)
            && crate::ipc::deferred::mailboxes_quiescent();
        passed &= reclaimed;
        crate::event!(
            "{{\"event\":\"ipc-request-quota\",\"status\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"requests_limit\":0,\"exhausted\":true,\"reclaimed\":{}}}",
            if passed { "pass" } else { "fail" },
            client_cpu,
            service_cpu,
            reclaimed
        );
    }
    report("ipc_request_quota_failure_has_no_phantom_work", passed);
}
fn requester_death(
    physical: &mut memory::Physical,
    registry: &mut Registry,
    report: &mut impl FnMut(&str, bool),
) {
    let start = &raw const ipc_death_image_start as usize;
    let end = &raw const ipc_death_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact bounded linked immutable fixture extent.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    for (name, service_role) in [
        ("ipc_requester_death_queued", 3),
        ("ipc_requester_death_delivered", 1),
        ("ipc_requester_death_committed", 2),
    ] {
        let mut passed = true;
        for (client_cpu, service_cpu) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
            let client = create_peer(registry, physical, image, client_cpu);
            let service = create_peer(registry, physical, image, service_cpu);
            let survivor = create_peer(registry, physical, image, 1 - client_cpu);
            let domains = [
                registry.domain_reference(client),
                registry.domain_reference(service),
                registry.domain_reference(survivor),
            ];
            let (work, work_receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
            let (control, control_receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
            let (notice, notice_receiver) = registry.bootstrap_endpoint(survivor, 1).unwrap();
            let work_client = registry
                .bootstrap_sender(client, &work, Rights::SEND)
                .unwrap();
            let work_survivor = registry
                .bootstrap_sender(survivor, &work, Rights::SEND)
                .unwrap();
            let control_sender = registry
                .bootstrap_sender(service, &control, Rights::SEND)
                .unwrap();
            let notice_sender = registry
                .bootstrap_sender(service, &notice, Rights::SEND)
                .unwrap();
            registry.ipc_input(client, 0, work_client);
            registry.ipc_control_input(client, control_receiver);
            registry.ipc_input(service, service_role, work_receiver);
            registry.ipc_control_input(service, control_sender);
            registry.ipc_survivor_input(service, notice_sender);
            registry.ipc_input(survivor, 4, work_survivor);
            registry.ipc_control_input(survivor, notice_receiver);
            for id in [client, service, survivor] {
                registry.start(id).unwrap();
            }
            let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
            for id in [client, service, survivor] {
                let task = &completed.tasks[id.slot()];
                let exit = registry.completion(id).unwrap().reason == Reason::Exited(1);
                passed &= exit && task.ipc_blocks == task.ipc_wakes;
                if !exit {
                    crate::event!(
                        "{{\"event\":\"ipc-fixture-failure\",\"slot\":{},\"state\":{},\"code\":{},\"fault\":{}}}",
                        id.slot(),
                        task.state,
                        task.context.gpr[0],
                        task.fault_class
                    );
                }
                registry.reclaim(physical, id).unwrap();
            }
            drop(work);
            drop(control);
            drop(notice);
            crate::ipc::reap();
            let reclaimed = completed.owners_released
                && physical.available() == before
                && domains
                    .iter()
                    .all(|domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0)
                && crate::ipc::deferred::mailboxes_quiescent();
            passed &= reclaimed;
            crate::event!(
                "{{\"event\":\"ipc-requester-death\",\"status\":\"{}\",\"name\":\"{}\",\"client_cpu\":{},\"service_cpu\":{},\"survivor_cpu\":{},\"reclaimed\":{},\"survivor_completed\":true}}",
                if passed { "pass" } else { "fail" },
                name,
                client_cpu,
                service_cpu,
                1 - client_cpu,
                reclaimed
            );
        }
        report(name, passed);
    }
}
