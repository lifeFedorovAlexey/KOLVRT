//! Development-only static image/manifest assurance, not production trust.
use crate::{
    cpu::context::Context,
    memory,
    process::{ImageFormat, Origin, Reason, Registry, Spec},
    supervision::{Grant, Scope},
};
use kernel_core::domain::Limits;
// SAFETY: INV-USER-IMAGE: permanent bounded immutable position-independent images.
core::arch::global_asm!(
    include_str!("supervision_workload.S"),
    commit_negative = const cfg!(feature = "supervision-commit-negative") as u8,
    dependency_negative = const cfg!(feature = "supervision-dependency-negative") as u8,
);
unsafe extern "C" {
    static supervisor_image_start: u8;
    static supervisor_image_end: u8;
    static supervised_service_start: u8;
    static supervised_service_end: u8;
}
fn image(start: *const u8, end: *const u8) -> &'static [u8] {
    let bytes = end as usize - start as usize;
    assert!(bytes > 0 && bytes <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: checked extent of permanent immutable linked image.
    unsafe { core::slice::from_raw_parts(start, bytes) }
}
pub(crate) fn exercise(physical: &mut memory::Physical, registry: &mut Registry) -> bool {
    let before = physical.available();
    let supervisor_image = image(
        &raw const supervisor_image_start,
        &raw const supervisor_image_end,
    );
    let service_image = image(
        &raw const supervised_service_start,
        &raw const supervised_service_end,
    );
    let limits = Limits {
        memory_pages: memory::USER_SPACE_PAGES,
        handles: 4,
        queue: 1,
        requests: 2,
        endpoints: 1,
    };
    let mut context = Context::ZERO;
    context.pc = memory::USER_CODE as u64;
    context.sp = memory::USER_STACK_TOP as u64;
    let supervisor = registry
        .create(
            physical,
            Origin::Bootstrap,
            Spec {
                image: supervisor_image,
                image_format: ImageFormat::RawFixture,
                limits: Limits {
                    endpoints: 2,
                    ..limits
                },
                context,
                owner: 0,
                entry: memory::USER_CODE,
                slice_limit: None,
            },
        )
        .unwrap();
    let grants = [
        Grant {
            image: service_image,
            image_format: ImageFormat::RawFixture,
            send_to: None,
            owner: 1,
            limits,
            instances: 8,
        },
        Grant {
            image: service_image,
            image_format: ImageFormat::RawFixture,
            send_to: None,
            owner: 0,
            limits,
            instances: 1,
        },
        Grant {
            image: service_image,
            image_format: ImageFormat::RawFixture,
            send_to: None,
            owner: 1,
            limits,
            instances: 1,
        },
    ];
    let mut scope = Scope::install(supervisor, grants);
    registry.start(supervisor).unwrap();
    let watchdog = crate::time::deadline_after(crate::time::Duration::from_secs(10));
    let completed = loop {
        assert!(
            crate::cpu::ticks() < watchdog,
            "supervision fixture watchdog; not lifecycle acceptance"
        );
        let completed = registry.checkpoint();
        if registry.completion(supervisor).is_ok() {
            break completed;
        }
        scope.service(registry, physical);
    };
    let reason = registry.completion(supervisor).unwrap().reason;
    let coverage = completed.tasks[supervisor.slot()].context.gpr[12];
    if reason != Reason::Exited(1) || coverage != 255 {
        let diagnostic = &completed.tasks[supervisor.slot()].context.gpr;
        crate::event!(
            "{{\"event\":\"supervision-reject\",\"status\":\"fail\",\"error\":\"{}\",\"exit\":\"{:?}\",\"coverage\":{},\"actual\":{},\"value\":{},\"stage\":{},\"operation\":{},\"submitted_at\":{},\"deadline\":{},\"failed_at\":{},\"frequency\":{}}}",
            if reason == Reason::Exited(1100) {
                "CommitNotProven"
            } else if reason == Reason::Exited(1200) {
                "DependencyNotRejected"
            } else {
                "Scenario"
            },
            reason,
            coverage,
            completed.tasks[supervisor.slot()].context.gpr[29],
            completed.tasks[supervisor.slot()].context.gpr[1],
            diagnostic[20],
            diagnostic[10],
            diagnostic[7],
            diagnostic[6],
            diagnostic[5],
            diagnostic[8]
        );
        let peer = completed.tasks[crate::scheduler::TASKS];
        crate::event!(
            "{{\"event\":\"supervision-peer\",\"state\":{},\"pc\":{},\"blocks\":{},\"wakes\":{},\"slices\":{},\"arg\":{},\"x0\":{},\"x1\":{}}}",
            peer.state,
            peer.context.pc,
            peer.ipc_blocks,
            peer.ipc_wakes,
            peer.slices,
            peer.context.gpr[23],
            peer.context.gpr[0],
            peer.context.gpr[1]
        );
        panic!("supervision exact EL0 scenario rejection");
    }
    scope.retire(registry, physical);
    registry.reclaim(physical, supervisor).unwrap();
    // Drain stale/dead wake acknowledgements using an empty owned checkpoint.
    registry.checkpoint();
    crate::ipc::deferred::poll(&[]);
    crate::ipc::reap();
    let passed = reason == Reason::Exited(1)
        && coverage == 255
        && completed.owners_released
        && physical.available() == before
        && registry.live() == 0
        && kernel_core::domain::live_domains() == 0;
    crate::event!(
        "{{\"event\":\"supervision\",\"status\":\"{}\",\"coverage\":{},\"reclaimed\":{},\"owners_released\":{}}}",
        if passed { "pass" } else { "fail" },
        coverage,
        physical.available() == before,
        completed.owners_released
    );
    passed
}
