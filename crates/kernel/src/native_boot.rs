//! Generic development bootstrap of immutable standalone ELF images.
//! Kernel mechanisms do not parse application reports or counter protocol.
use crate::{
    cpu,
    cpu::context::Context,
    memory,
    platform::config,
    process::{ImageFormat, Origin, Reason, Registry, Spec},
    supervision::{Grant, Scope},
};
use kernel_core::domain::Limits;

pub(crate) fn exercise(physical: &mut memory::Physical, registry: &mut Registry) {
    let before = physical.available();
    let limits = Limits {
        memory_pages: 64,
        handles: 8,
        queue: 1,
        requests: 4,
        endpoints: 1,
    };
    let root = include_bytes!(env!("KOLVRT_NATIVE_ROOT_ELF"));
    let service = include_bytes!(env!("KOLVRT_NATIVE_SERVICE_ELF"));
    let client = include_bytes!(env!("KOLVRT_NATIVE_CLIENT_ELF"));
    let mut context = Context::ZERO;
    context.pc = config::USER_PAYLOAD_BASE as u64;
    context.sp = memory::USER_STACK_TOP as u64;
    context.gpr[23] = env!("KOLVRT_NATIVE_ARGUMENT")
        .parse()
        .expect("opaque root argument");
    let supervisor = registry
        .create(
            physical,
            Origin::Bootstrap,
            Spec {
                image: root,
                image_format: ImageFormat::Elf64Aarch64,
                limits: Limits {
                    handles: 16,
                    endpoints: 3,
                    ..limits
                },
                context,
                owner: 0,
                entry: config::USER_PAYLOAD_BASE,
                slice_limit: None,
            },
        )
        .unwrap_or_else(|failure| {
            crate::event!(
                "{{\"event\":\"native-image-reject\",\"error\":\"{:?}\",\"had_completion\":{}}}",
                failure.error,
                failure.completion.is_some()
            );
            cpu::poweroff()
        });
    crate::event!(
        "{{\"event\":\"native-root-image\",\"format\":\"elf64\",\"slot\":{},\"generation\":{}}}",
        supervisor.slot(),
        supervisor.generation()
    );
    let mut scope = Scope::install(
        supervisor,
        [
            Grant {
                image: service,
                image_format: ImageFormat::Elf64Aarch64,
                send_to: None,
                owner: 1,
                limits,
                instances: 4,
            },
            Grant {
                image: client,
                image_format: ImageFormat::Elf64Aarch64,
                send_to: Some(0),
                owner: 0,
                limits,
                instances: 1,
            },
            Grant {
                image: client,
                image_format: ImageFormat::Elf64Aarch64,
                send_to: None,
                owner: 1,
                limits,
                instances: 1,
            },
        ],
    );
    registry.start(supervisor).expect("root admission");
    let mut published_words = 0;
    let completed = loop {
        let completed = registry.checkpoint();
        {
            let (words, length) = completed.report_chunk(supervisor.slot(), 0);
            if length > published_words {
                crate::event!(
                    "{{\"event\":\"native-report-progress\",\"words\":{:?}}}",
                    &words[..length]
                );
                published_words = length;
            }
        }
        if registry.completion(supervisor).is_ok() {
            break completed;
        }
        scope.service(registry, physical);
    };
    for task in completed
        .tasks
        .iter()
        .filter(|task| task.process_generation != 0)
    {
        crate::event!(
            "{{\"event\":\"native-task\",\"generation\":{},\"state\":{},\"ipc_blocks\":{},\"terminal_blocks\":{},\"ipc_wakes\":{},\"peer_faults_at_exit\":{}}}",
            task.process_generation,
            task.state,
            task.ipc_blocks,
            task.ipc_terminal_blocks,
            task.ipc_wakes,
            task.peer_faults_at_exit
        );
    }
    crate::event!(
        "{{\"event\":\"native-scheduler\",\"all_blocked\":{}}}",
        completed.ipc_all_blocked
    );
    let reason = registry
        .completion(supervisor)
        .expect("root completion")
        .reason;
    // Read only the completed report; no new storage or application interpretation.
    let mut total = 0;
    loop {
        let (_, length) = completed.report_chunk(supervisor.slot(), total);
        total += length;
        if length < crate::scheduler::REPORT_CHUNK_WORDS {
            break;
        }
    }
    if total <= crate::scheduler::REPORT_CHUNK_WORDS {
        let (words, length) = completed.report_chunk(supervisor.slot(), 0);
        crate::event!(
            "{{\"event\":\"native-user-report\",\"words\":{:?}}}",
            &words[..length]
        );
    } else {
        let mut offset = 0;
        while offset < total {
            let (words, length) = completed.report_chunk(supervisor.slot(), offset);
            crate::event!(
                "{{\"event\":\"native-user-report-chunk\",\"version\":2,\"slot\":{},\"generation\":{},\"offset\":{},\"total\":{},\"words\":{:?}}}",
                supervisor.slot(),
                supervisor.generation(),
                offset,
                total,
                &words[..length]
            );
            offset += length;
        }
        crate::event!(
            "{{\"event\":\"native-user-report-end\",\"version\":2,\"slot\":{},\"generation\":{},\"total\":{}}}",
            supervisor.slot(),
            supervisor.generation(),
            total
        );
    }
    if reason != Reason::Exited(0) {
        let (kind, detail) = match reason {
            Reason::Exited(code) => ("exit", code),
            Reason::Faulted { class, .. } => (
                if class == crate::cpu::context::ESR_SYSREG_TRAP {
                    "sysreg-fault"
                } else if class == crate::cpu::context::ESR_UNKNOWN {
                    "instruction-fault"
                } else {
                    "fault"
                },
                class,
            ),
            Reason::BudgetExpired => ("budget", 0),
            Reason::Terminated => ("terminated", 0),
            Reason::CreationFailed(_) => ("creation", 0),
        };
        crate::event!(
            "{{\"event\":\"native-boot\",\"status\":\"root-failed\",\"kind\":\"{}\",\"detail\":{},\"resources_retained\":true}}",
            kind,
            detail
        );
        cpu::poweroff();
    }
    scope.retire(registry, physical);
    registry
        .reclaim(physical, supervisor)
        .expect("root retirement");
    registry.checkpoint();
    crate::ipc::deferred::poll(&[]);
    crate::ipc::reap();
    let restored = physical.available() == before;
    let processes = registry.live();
    let domains = kernel_core::domain::live_domains();
    crate::event!(
        "{{\"event\":\"native-boot\",\"status\":\"complete\",\"root_exit\":0,\"owners_released\":{},\"frames_restored\":{},\"live_processes\":{},\"live_domains\":{}}}",
        completed.owners_released,
        restored,
        processes,
        domains
    );
    assert!(
        restored && processes == 0 && domains == 0,
        "native generic resource reclamation"
    );
    crate::smp::shutdown();
}
