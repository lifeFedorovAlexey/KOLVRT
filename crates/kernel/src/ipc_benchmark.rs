//! QEMU regression baseline, never a hardware speed target. EL0 clock-probe
//! envelopes include the probe's return/entry cost and user argument setup.
use crate::{
    cpu, memory,
    process::{ImageFormat, Origin, Reason, Registry, Spec},
};
use kernel_core::{domain::Limits, handles::Rights};
// SAFETY: INV-USER-IMAGE: bounded immutable position-independent measurement image.
core::arch::global_asm!(include_str!("ipc_benchmark.S"));
// SAFETY: INV-USER-IMAGE: bounded immutable position-independent path fixtures.
core::arch::global_asm!(include_str!("ipc_paths_benchmark.S"));
unsafe extern "C" {
    static ipc_benchmark_image_start: u8;
    static ipc_benchmark_image_end: u8;
    static ipc_paths_benchmark_image_start: u8;
    static ipc_paths_benchmark_image_end: u8;
}
pub(crate) fn exercise(physical: &mut memory::Physical, registry: &mut Registry) {
    let start = &raw const ipc_benchmark_image_start as usize;
    let end = &raw const ipc_benchmark_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact checked immutable linked image bytes.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let start = &raw const ipc_paths_benchmark_image_start as usize;
    let end = &raw const ipc_paths_benchmark_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: exact checked immutable linked path image bytes.
    let paths = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let before = physical.available();
    for (client_cpu, service_cpu) in [(0, 0), (1, 1), (0, 1), (1, 0)] {
        for payload in [0, 8, 64, 256] {
            for (kind, scope) in [
                (1, "submit"),
                (2, "collect"),
                (3, "round_trip"),
                (4, "receive"),
                (5, "reply"),
                (6, "hot_ready_receive"),
                (7, "blocked_receive_wake"),
                (8, "blocked_requester_wait"),
                (9, "queue_full_rejection"),
            ] {
                let image = if kind <= 5 { image } else { paths };
                let mut ids = [None, None];
                for (index, owner) in [client_cpu, service_cpu].into_iter().enumerate() {
                    let mut context = cpu::context::Context::ZERO;
                    context.pc = memory::USER_CODE as u64;
                    context.sp = memory::USER_STACK_TOP as u64;
                    ids[index] = Some(
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
                            )
                            .unwrap(),
                    );
                }
                let client = ids[0].unwrap();
                let service = ids[1].unwrap();
                let (endpoint, receiver) = registry.bootstrap_endpoint(service, 4).unwrap();
                let sender = registry
                    .bootstrap_sender(client, &endpoint, Rights::SEND)
                    .unwrap();
                let gate = if kind == 6 || kind == 9 {
                    let (gate, receiver) = registry.bootstrap_endpoint(client, 1).unwrap();
                    let sender = registry
                        .bootstrap_sender(service, &gate, Rights::SEND)
                        .unwrap();
                    registry.ipc_control_input(client, receiver);
                    registry.ipc_control_input(service, sender);
                    Some(gate)
                } else {
                    None
                };
                let domains = [
                    registry.domain_reference(client),
                    registry.domain_reference(service),
                ];
                registry.ipc_input(client, 0, sender);
                registry.ipc_bench_input(client, payload, kind);
                registry.ipc_input(service, 1, receiver);
                registry.ipc_bench_input(service, payload, kind);
                registry.start(client).unwrap();
                registry.start(service).unwrap();
                let completed = registry.dispatch_ipc(Some(crate::time::Duration::from_secs(5)));
                let mut passed = true;
                for id in [client, service] {
                    if registry.completion(id).unwrap().reason != Reason::Exited(1) {
                        let task = &completed.tasks[id.slot()];
                        crate::event!(
                            "{{\"event\":\"ipc-benchmark-failure\",\"kind\":{},\"payload\":{},\"slot\":{},\"state\":{},\"code\":{},\"actual\":{},\"value\":{},\"iteration\":{},\"block_delta\":{}}}",
                            kind,
                            payload,
                            id.slot(),
                            task.state,
                            task.context.gpr[0],
                            task.context.gpr[29],
                            task.context.gpr[1],
                            task.context.gpr[18],
                            task.context.gpr[4]
                        );
                        passed = false;
                    }
                }
                assert!(passed, "IPC benchmark peer failed");
                let measured = if kind <= 3 || kind >= 8 {
                    client
                } else {
                    service
                };
                assert_eq!(completed.tasks[measured.slot()].report_len, 20);
                let (words, length) = completed.report_chunk(measured.slot(), 4);
                assert_eq!(length, 16);
                let mut samples = [0u64; 16];
                samples.copy_from_slice(&words[..16]);
                let mut sorted = samples;
                let [median, p95, p99] = kernel_core::quantiles(&mut sorted).unwrap();
                crate::event!(
                    "{{\"event\":\"ipc-measurement\",\"scope\":\"{}\",\"controlled_path\":{},\"client_cpu\":{},\"service_cpu\":{},\"payload\":{},\"units\":\"timer_ticks\",\"frequency\":{},\"warmup\":4,\"iterations\":16,\"median\":{},\"p95\":{},\"p99\":{},\"samples\":{:?},\"context_switches\":{},\"client_blocks\":{},\"service_blocks\":{}}}",
                    scope,
                    kind >= 6,
                    client_cpu,
                    service_cpu,
                    payload,
                    cpu::frequency(),
                    median,
                    p95,
                    p99,
                    samples,
                    completed.switches,
                    completed.tasks[client.slot()].ipc_blocks,
                    completed.tasks[service.slot()].ipc_blocks
                );
                for id in [client, service] {
                    registry.reclaim(physical, id).unwrap();
                }
                drop(endpoint);
                drop(gate);
                crate::ipc::reap();
                assert_eq!(physical.available(), before);
                assert!(
                    domains.iter().all(
                        |domain| domain.usage() == (0, 0, 0, 0) && domain.endpoint_usage() == 0
                    )
                );
            }
        }
    }
}
