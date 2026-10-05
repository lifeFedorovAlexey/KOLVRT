//! Kernel integration fixture: actual EL0 clients, revoker and service on both CPUs.
use crate::{
    cpu::context::Context,
    memory,
    process::{ImageFormat, Origin, Reason, Registry, Spec},
};
use core::sync::atomic::{AtomicU64, Ordering};
use kernel_core::{
    domain::{Limits, Reference},
    handles::{Namespace, Rights},
    process::ProcessId,
    wait::SharedEvent,
};
static DONE: [AtomicU64; crate::platform::config::ACTIVE_CPUS] =
    [const { AtomicU64::new(0) }; crate::platform::config::ACTIVE_CPUS];
static RACE: AtomicU64 = AtomicU64::new(0);
// SAFETY: INV-USER-IMAGE: immutable position-independent EL0 fixture with checked extent.
core::arch::global_asm!(include_str!("testing.S"));
unsafe extern "C" {
    static security_image_start: u8;
    static security_image_end: u8;
}
pub(super) fn helper(table: &Namespace<8>, frame: &mut Context, operation: u16) -> bool {
    if operation == 0x93 {
        let flag = &DONE[crate::percpu::id()];
        if frame.gpr[0] != 0 {
            flag.store(1, Ordering::Release);
        }
        frame.gpr[0] = flag.load(Ordering::Acquire);
        return true;
    }
    if operation == 0x92 {
        let value = frame.gpr[0];
        if value != 0 {
            RACE.fetch_max(value, Ordering::AcqRel);
        }
        frame.gpr[0] = RACE.load(Ordering::Acquire);
        return true;
    }
    if operation != 0x91 {
        return false;
    }
    let mut ids = kernel_core::process::Table::<2>::new();
    let a = ids.reserve(0..1).unwrap();
    let b = ids.reserve(1..2).unwrap();
    let current = table.owner().unwrap();
    let foreign = if a != current { a } else { b };
    let domain = table.domain().unwrap();
    let mut forged = Namespace::<1>::new();
    frame.gpr[0] = u64::from(
        domain.validate(foreign).is_err() && forged.bind_domain(foreign, domain.clone()).is_err(),
    );
    true
}
fn spec(image: &[u8], owner: usize) -> Spec<'_> {
    let mut context = Context::ZERO;
    context.pc = memory::USER_CODE as u64;
    context.sp = memory::USER_STACK_TOP as u64;
    context.gpr[20] = 3;
    Spec {
        image,
        image_format: ImageFormat::RawFixture,
        limits: limits(),
        context,
        owner,
        entry: memory::USER_CODE,
        slice_limit: None,
    }
}
fn create(p: &mut memory::Physical, r: &mut Registry, image: &[u8], owner: usize) -> ProcessId {
    r.create(p, Origin::Bootstrap, spec(image, owner), None)
        .unwrap()
}
fn passed(r: &Registry, id: ProcessId) -> bool {
    r.completion(id)
        .is_ok_and(|c| c.reason == Reason::Exited(1))
}
pub(crate) fn exercise(
    p: &mut memory::Physical,
    r: &mut Registry,
    mut report: impl FnMut(&str, bool),
) {
    let before = p.available();
    let start = &raw const security_image_start as usize;
    let end = &raw const security_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: bounded immutable linked EL0 instruction fixture.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let limits = Limits {
        memory_pages: 1,
        handles: 1,
        queue: 1,
        requests: 1,
        endpoints: 0,
    };
    let memory_denied = match r.create_bounded(p, Origin::Bootstrap, spec(image, 0), None, limits) {
        Err(_) => p.available() == before,
        Ok(id) => {
            r.start(id).unwrap();
            r.dispatch(Some(crate::time::Duration::from_secs(5)));
            r.reclaim(p, id).unwrap();
            false
        }
    };
    report("domain_memory_budget_enforced", memory_denied);
    let mut quota_denied = true;
    for request_zero in [true, false] {
        for flag in &DONE {
            flag.store(0, Ordering::Release);
        }
        let mut ids = [None; crate::process::CAPACITY];
        let mut events: [Option<SharedEvent>; crate::platform::config::ACTIVE_CPUS] =
            core::array::from_fn(|_| None);
        for (owner, event_cell) in events.iter_mut().enumerate() {
            let limits = |requests, queue| Limits {
                memory_pages: memory::USER_SPACE_PAGES,
                handles: 2,
                queue,
                requests,
                endpoints: 0,
            };
            let client = r
                .create_bounded(
                    p,
                    Origin::Bootstrap,
                    spec(image, owner),
                    None,
                    limits(if request_zero { 0 } else { 1 }, 1),
                )
                .unwrap();
            let server = r
                .create_bounded(
                    p,
                    Origin::Bootstrap,
                    spec(image, owner),
                    None,
                    limits(1, if request_zero { 1 } else { 0 }),
                )
                .unwrap();
            let event = SharedEvent::try_new().unwrap();
            let grant = r
                .bootstrap_grant(client, server, event.try_clone().unwrap(), Rights::ISSUER)
                .unwrap();
            r.security_input(client, [6, grant.encode(), 0, 0, 0, 0, 0]);
            r.security_input(server, [7, 0, 0, 0, 0, 0, 0]);
            ids[client.slot()] = Some(client);
            ids[server.slot()] = Some(server);
            *event_cell = Some(event);
            r.start(client).unwrap();
            r.start(server).unwrap();
        }
        r.dispatch(Some(crate::time::Duration::from_secs(5)));
        for id in ids.into_iter().flatten() {
            quota_denied &= passed(r, id);
            r.reclaim(p, id).unwrap();
        }
        quota_denied &= events.iter().flatten().all(|e| e.register()) && p.available() == before;
    }
    report("domain_el0_request_and_queue_budgets", quota_denied);
    let mut integration = true;
    let mut fault_contained = true;
    let mut rebind = true;
    let mut previous: Option<(ProcessId, Reference)> = None;
    for fault in [false, true, false] {
        for flag in &DONE {
            flag.store(0, Ordering::Release);
        }
        let mut ids = [None; crate::process::CAPACITY];
        let mut events: [Option<SharedEvent>; crate::platform::config::ACTIVE_CPUS] =
            core::array::from_fn(|_| None);
        let mut retained: [Option<Reference>; crate::platform::config::ACTIVE_CPUS] =
            core::array::from_fn(|_| None);
        for owner in 0..crate::platform::config::ACTIVE_CPUS {
            let client = create(p, r, image, owner);
            let server = create(p, r, image, owner);
            let limited = r
                .create_bounded(
                    p,
                    Origin::Bootstrap,
                    spec(image, owner),
                    None,
                    Limits {
                        memory_pages: memory::USER_SPACE_PAGES,
                        handles: 0,
                        queue: 1,
                        requests: 1,
                        endpoints: 0,
                    },
                )
                .unwrap();
            if owner == 0 {
                if let Some((old, ref old_domain)) = previous {
                    rebind &= old != client
                        && old_domain.id() != r.domain_reference(client).id()
                        && old_domain.closing()
                        && old_domain.usage() == (0, 0, 0, 0);
                }
                previous = Some((client, r.domain_reference(client)));
            }
            let event = SharedEvent::try_new().unwrap();
            let grant = r
                .bootstrap_grant(client, server, event.try_clone().unwrap(), Rights::ISSUER)
                .unwrap();
            let readonly = r
                .bootstrap_grant(
                    client,
                    server,
                    SharedEvent::try_new().unwrap(),
                    Rights::SEND,
                )
                .unwrap();
            let no_rights = r
                .bootstrap_grant(
                    client,
                    server,
                    SharedEvent::try_new().unwrap(),
                    Rights::NONE,
                )
                .unwrap();
            let private = r
                .bootstrap_grant(
                    server,
                    client,
                    SharedEvent::try_new().unwrap(),
                    Rights::ISSUER,
                )
                .unwrap();
            let delegated = r
                .transfer_handle(client, grant, server, Rights::SEND)
                .unwrap();
            r.security_input(
                client,
                [
                    if fault { 2 } else { 0 },
                    grant.encode(),
                    no_rights.encode(),
                    readonly.encode(),
                    memory::UserSpace::alias(server.slot()) as u64,
                    limited.slot() as u64,
                    limited.generation(),
                ],
            );
            r.security_input(
                server,
                [1, private.encode(), delegated.encode(), 0, 0, 0, 0],
            );
            r.security_input(limited, [7, 0, 0, 0, 0, 0, 0]);
            retained[owner] = Some(r.domain_reference(client));
            events[owner] = Some(event);
            for id in [client, server, limited] {
                ids[id.slot()] = Some(id);
                r.start(id).unwrap();
            }
        }
        r.dispatch(Some(crate::time::Duration::from_secs(5)));
        for owner in 0..crate::platform::config::ACTIVE_CPUS {
            let base = owner * crate::scheduler::TASKS;
            let client = ids[base].unwrap();
            let server = ids[base + 1].unwrap();
            let peer = ids[base + 2].unwrap();
            #[cfg(feature = "diagnostics")]
            crate::diagnostics::status(
                "DEBUG",
                "security-fixture",
                format_args!(
                    "fault={} cpu={} client={:?} server={:?} peer={:?}",
                    fault,
                    owner,
                    r.completion(client),
                    r.completion(server),
                    r.completion(peer)
                ),
            );
            integration &=
                passed(r, server) && passed(r, peer) && !events[owner].as_ref().unwrap().register();
            fault_contained &= if fault {
                r.completion(client)
                    .is_ok_and(|c| matches!(c.reason, Reason::Faulted { .. }))
            } else {
                passed(r, client)
            };
            for id in [client, server, peer] {
                r.reclaim(p, id).unwrap();
            }
            integration &= retained[owner].as_ref().unwrap().closing()
                && retained[owner].as_ref().unwrap().usage() == (0, 0, 0, 0);
        }
        integration &= p.available() == before;
        if !integration || !fault_contained {
            report("capability_el0_scope_attenuation_and_denial", false);
        }
    }
    report(
        "capability_el0_scope_attenuation_and_denial",
        integration && fault_contained,
    );
    report("capability_revocation_retains_admitted_effect", integration);
    report("domain_teardown_retains_accepted_notification", integration);
    report(
        "domain_fault_peer_progress_and_reclamation",
        fault_contained && integration,
    );
    report("domain_rebind_does_not_restore_grants", rebind);
    retained_after_reclaim(p, r, image, &mut report);
    service_rebind_and_cancellation(p, r, image, &mut report);
    RACE.store(0, Ordering::Release);
    let client = create(p, r, image, 0);
    let server = create(p, r, image, 0);
    let issuer = create(p, r, image, 1);
    let peer = create(p, r, image, 1);
    let event = SharedEvent::try_new().unwrap();
    let grant = r
        .bootstrap_grant(issuer, server, event.try_clone().unwrap(), Rights::ISSUER)
        .unwrap();
    let received = r
        .transfer_handle(issuer, grant, client, Rights::SEND)
        .unwrap();
    r.security_input(client, [5, received.encode(), 0, 0, 0, 0, 0]);
    r.security_input(server, [1, grant.encode(), 0, 0, 0, 0, 0]);
    r.security_input(issuer, [4, grant.encode(), 0, 0, 0, 0, 0]);
    for id in [client, server, issuer, peer] {
        r.start(id).unwrap();
    }
    r.dispatch(Some(crate::time::Duration::from_secs(5)));
    let race = [client, server, issuer, peer]
        .iter()
        .all(|id| passed(r, *id))
        && !event.register();
    for id in [client, server, issuer, peer] {
        r.reclaim(p, id).unwrap();
    }
    report(
        "capability_cross_cpu_revoke_admission",
        race && p.available() == before,
    );
}

fn retained_after_reclaim(
    p: &mut memory::Physical,
    r: &mut Registry,
    image: &[u8],
    report: &mut impl FnMut(&str, bool),
) {
    let pages = p.available();
    let domains = kernel_core::domain::live_domains();
    let mut retained = true;
    let mut cleanup = true;
    for service_fault in [false, true] {
        let client = create(p, r, image, 0);
        let server = create(p, r, image, 0);
        let event = SharedEvent::try_new().unwrap();
        let grant = r
            .bootstrap_grant(client, server, event.try_clone().unwrap(), Rights::ISSUER)
            .unwrap();
        r.security_input(client, [9, grant.encode(), 0, 0, 0, 0, 0]);
        r.security_input(
            server,
            [
                if service_fault { 11 } else { 8 },
                grant.encode(),
                0,
                0,
                0,
                0,
                0,
            ],
        );
        let domain = r.domain_reference(client);
        r.start(client).unwrap();
        r.start(server).unwrap();
        let end = crate::time::deadline_after(crate::time::Duration::from_secs(5));
        while r.completion(client).is_err() || !r.blocked(server).unwrap_or(false) {
            assert!(crate::cpu::ticks() < end, "security step fixture timeout");
            r.step();
        }
        retained &= passed(r, client) && domain.closing() && domain.usage().3 == 1;
        r.reclaim(p, client).unwrap();
        retained &=
            domain.usage() == (0, 0, 0, 1) && kernel_core::domain::live_domains() == domains + 2;
        drop(domain); // Accepted Work is now the last owner of the consumer domain.
        r.signal(server).unwrap();
        while r.completion(server).is_err() {
            assert!(
                crate::cpu::ticks() < end,
                "security service fixture timeout"
            );
            r.step();
        }
        cleanup &= if service_fault {
            r.completion(server)
                .is_ok_and(|c| matches!(c.reason, Reason::Faulted { .. }))
                && event.register()
        } else {
            passed(r, server) && !event.register()
        };
        r.reclaim(p, server).unwrap();
        cleanup &= p.available() == pages && kernel_core::domain::live_domains() == domains;
    }
    report("domain_reclaimed_sender_retains_request_charge", retained);
    report(
        "domain_service_fault_cancels_effect_and_releases_charges",
        cleanup,
    );
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

fn service_rebind_and_cancellation(
    p: &mut memory::Physical,
    r: &mut Registry,
    image: &[u8],
    report: &mut impl FnMut(&str, bool),
) {
    let pages = p.available();
    let domains = kernel_core::domain::live_domains();
    let mut rebind = true;
    let mut cancellation = true;
    for service_fault in [false, true] {
        let client = create(p, r, image, 0);
        let server = create(p, r, image, 0);
        let event = SharedEvent::try_new().unwrap();
        let grant = r
            .bootstrap_grant(client, server, event.try_clone().unwrap(), Rights::ISSUER)
            .unwrap();
        r.security_input(
            client,
            [
                if service_fault { 14 } else { 12 },
                grant.encode(),
                0,
                0,
                0,
                0,
                0,
            ],
        );
        r.security_input(
            server,
            [
                if service_fault { 11 } else { 13 },
                grant.encode(),
                0,
                0,
                0,
                0,
                0,
            ],
        );
        r.start(client).unwrap();
        r.start(server).unwrap();
        let end = crate::time::deadline_after(crate::time::Duration::from_secs(5));
        if service_fault {
            while !r.blocked(client).unwrap_or(false) || !r.blocked(server).unwrap_or(false) {
                assert!(crate::cpu::ticks() < end, "notification block timeout");
                r.step();
            }
            r.signal(server).unwrap();
        }
        while r.completion(server).is_err() || !r.blocked(client).unwrap_or(false) {
            assert!(crate::cpu::ticks() < end, "notification terminal timeout");
            r.step();
        }
        let effect = !event.register();
        let server_passed = if service_fault {
            r.completion(server)
                .is_ok_and(|c| matches!(c.reason, Reason::Faulted { .. }))
        } else {
            passed(r, server)
        };
        r.reclaim(p, server).unwrap();
        let replacement = if !service_fault {
            let id = create(p, r, image, 0);
            rebind &= id.slot() == server.slot() && id.generation() != server.generation();
            r.start(id).unwrap();
            Some(id)
        } else {
            None
        };
        r.signal(client).unwrap();
        while r.completion(client).is_err()
            || replacement.is_some_and(|id| r.completion(id).is_err())
        {
            assert!(crate::cpu::ticks() < end, "notification client timeout");
            r.step();
        }
        if service_fault {
            cancellation &= passed(r, client) && server_passed && !effect;
        } else {
            rebind &= passed(r, client) && server_passed && effect;
        }
        r.reclaim(p, client).unwrap();
        if let Some(id) = replacement {
            r.reclaim(p, id).unwrap();
        }
        rebind &= p.available() == pages && kernel_core::domain::live_domains() == domains;
    }
    report("capability_service_rebind_preserves_scope", rebind);
    report(
        "domain_el0_observes_service_fault_cancellation",
        cancellation,
    );
}
