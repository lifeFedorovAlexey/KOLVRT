use super::*;
use crate::{
    domain::{Limits, Memory, Owner},
    handles::{Handle, Kind, Namespace, Rights},
};
use std::{
    sync::{Mutex, MutexGuard},
    vec::Vec,
};
static TEST_LOCK: Mutex<()> = Mutex::new(());
#[test]
fn closing_requester_is_observed_before_deferred_death_notification() {
    for committed in [false, true] {
        let mut rig = Rig::new(1);
        let service = rig.ids[0];
        rig.submit(1, 42, b"work").unwrap();
        let (token, _) = rig.receive();
        if committed {
            rig.ep().commit(service, token, 2).unwrap();
        }
        rig.domains[1].close();
        // No process_death callback yet: reproduce a skipped busy cell while
        // another endpoint has already published the dying process's outcome.
        if committed {
            assert_eq!(
                rig.ep().reply(service, token, Payload::EMPTY, 3),
                Ok(Outcome::Completed)
            );
        } else {
            assert_eq!(rig.ep().commit(service, token, 3), Err(Error::Stale));
            assert_eq!(
                rig.ep().reply(service, token, Payload::EMPTY, 3),
                Err(Error::Stale)
            );
        }
        assert_eq!(rig.ep().outstanding(), 0);
        assert_eq!(rig.usages()[1].3, 0);
    }
}

#[test]
fn accepted_receipt_is_exact_requester_scoped_and_survives_grant_revoke() {
    let mut rig = Rig::new(1);
    let client = rig.ids[1];
    let foreign = rig.ids[2];
    let request = rig.submit(1, 42, b"retained").unwrap();
    let value = request.receipt();
    assert_eq!(rig.ep().receipt(client, value), Ok(request));
    assert_eq!(rig.ep().receipt(foreign, value), Err(Error::Stale));
    assert_eq!(rig.ep().receipt(client, 0), Err(Error::Stale));
    rig.ep().reference().revoke();
    assert_eq!(rig.ep().receipt(client, value), Ok(request));
    rig.ep().cancel(client, request, 2).unwrap();
    assert_eq!(rig.ep().receipt(client, value), Ok(request));
    rig.collect(1, request);
    assert_eq!(rig.ep().receipt(client, value), Err(Error::Stale));
}

#[test]
fn service_token_cannot_be_used_by_another_process() {
    let mut rig = Rig::new(1);
    let service = rig.ids[0];
    let foreign = rig.ids[2];
    let request = rig.submit(1, 42, b"bound-token").unwrap();
    let (token, _) = rig.receive();
    let before = rig.usages();
    assert_eq!(rig.ep().commit(foreign, token, 2), Err(Error::Denied));
    assert_eq!(
        rig.ep().reply(foreign, token, Payload::EMPTY, 2),
        Err(Error::Denied)
    );
    assert_eq!(rig.usages(), before);
    assert_eq!(rig.ep().commit(service, token, 3), Ok(()));
    assert_eq!(
        rig.ep()
            .reply(service, token, Payload::copy(b"valid").unwrap(), 4),
        Ok(Outcome::Completed)
    );
    assert_eq!(rig.collect(1, request).1.bytes(), b"valid");
}

struct Rig {
    endpoint: Option<Endpoint>,
    ids: [ProcessId; 5],
    domains: [Owner; 5],
    _memory: [Memory; 5],
    _serial: MutexGuard<'static, ()>,
}
impl Rig {
    fn new(capacity: usize) -> Self {
        let serial = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut table = crate::process::Table::<5>::new();
        let ids = core::array::from_fn(|i| table.reserve(i..i + 1).unwrap());
        let pairs = ids.map(|id| {
            Owner::new(
                id,
                Limits {
                    memory_pages: 1,
                    handles: 8,
                    queue: 4,
                    requests: 8,
                    endpoints: u16::from(id == ids[0]),
                },
                1,
            )
            .unwrap()
        });
        let mut owners = Vec::new();
        let mut memory = Vec::new();
        for (owner, pages) in pairs {
            owners.push(owner);
            memory.push(pages);
        }
        let domains: [Owner; 5] = owners.try_into().ok().unwrap();
        let endpoint = Endpoint::create(domains[0].reference(), 0, capacity).unwrap();
        Self {
            endpoint: Some(endpoint),
            ids,
            domains,
            _memory: memory.try_into().ok().unwrap(),
            _serial: serial,
        }
    }
    fn ep(&mut self) -> &mut Endpoint {
        self.endpoint.as_mut().unwrap()
    }
    fn submit(&mut self, client: usize, id: u64, bytes: &[u8]) -> Result<RequestId, Error> {
        let consumer = self.domains[client].reference();
        let grant = self.ep().reference().try_clone()?;
        self.ep()
            .submit(&grant, &consumer, id, 1000, Payload::copy(bytes)?, 1)
    }
    fn receive(&mut self) -> (ServiceToken, Payload) {
        let service = self.ids[0];
        let delivery = self.ep().reserve_receive(service, 1).unwrap();
        self.ep().finish_receive(&delivery, true).unwrap();
        (delivery.token, delivery.payload)
    }
    fn collect(&mut self, client: usize, id: RequestId) -> (Outcome, Payload) {
        let caller = self.ids[client];
        let collection = self.ep().reserve_collect(caller, id).unwrap();
        self.ep().finish_collect(&collection, true).unwrap();
        (collection.outcome, collection.payload)
    }
    fn usages(&self) -> [(usize, u16, u16, u16); 5] {
        core::array::from_fn(|i| self.domains[i].reference().usage())
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        if let Some(mut endpoint) = self.endpoint.take() {
            endpoint.shutdown();
            for id in self.ids {
                endpoint.process_death(id);
            }
            let ready: Vec<_> = endpoint.pending_wakes().map(|w| w.key).collect();
            for key in ready {
                endpoint.acknowledge_wake(key).unwrap();
            }
            assert_eq!(endpoint.outstanding(), 0);
            assert_eq!(endpoint.occupancy(), 0);
            assert_eq!(endpoint.waiter_count(), 0);
            assert!(endpoint.reclaimable());
            drop(endpoint);
            for usage in self.usages() {
                assert_eq!(usage, (1, 0, 0, 0));
            }
            for domain in &self.domains {
                assert_eq!(domain.reference().endpoint_usage(), 0);
            }
        }
    }
}

#[test]
fn exact_payload_bounds_snapshot_and_initialized_frames() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    for length in [0, 1, 8, 64, 255, 256] {
        let mut source: Vec<_> = (0..length).map(|i| i as u8 ^ 0x5a).collect();
        let original = source.clone();
        let request = rig.submit(1, length as u64, &source).unwrap();
        source.fill(0xff);
        let delivery = rig.ep().reserve_receive(service, 1).unwrap();
        assert_eq!(delivery.payload.bytes(), original);
        let frame = wire::Output::received(
            delivery.token,
            delivery.client_id,
            delivery.deadline,
            &delivery.payload,
        );
        assert_eq!(&frame.bytes()[wire::HEADER..], original);
        assert_eq!(&frame.bytes()[2..4], &[0, 0]);
        assert_eq!(&frame.bytes()[36..40], &[0; 4]);
        rig.ep().finish_receive(&delivery, true).unwrap();
        let mut response = original.clone();
        response.reverse();
        let snapshot = Payload::copy(&response).unwrap();
        response.fill(0xcc);
        rig.ep()
            .reply(service, delivery.token, snapshot, 2)
            .unwrap();
        drop(delivery);
        let (outcome, payload) = rig.collect(1, request);
        assert_eq!(outcome, Outcome::Completed);
        let expected: Vec<_> = original.into_iter().rev().collect();
        assert_eq!(payload.bytes(), expected);
        let frame = wire::Output::result(length as u64, outcome, &payload);
        assert_eq!(&frame.bytes()[20..24], &[0; 4]);
        assert_eq!(&frame.bytes()[24..], expected);
    }
    assert_eq!(Payload::copy(&[1; 257]), Err(Error::Invalid));
    assert_eq!(rig.ep().outstanding(), 0);
}

#[test]
fn queue_capacity_fifo_full_rollback_and_reuse() {
    for capacity in [1, 4] {
        let mut rig = Rig::new(capacity);
        let service = rig.ids[0];
        for round in 0..32 {
            let mut accepted = Vec::new();
            for i in 1..=capacity {
                accepted.push(rig.submit(i, round, &[i as u8]).unwrap());
            }
            let before = rig.usages();
            assert_eq!(rig.submit(1, round + 1000, &[99]), Err(Error::Exhausted));
            assert_eq!(before, rig.usages());
            assert_eq!(rig.ep().occupancy(), capacity);
            for (index, request) in accepted.into_iter().enumerate() {
                let (token, payload) = rig.receive();
                assert_eq!(payload.bytes(), &[(index + 1) as u8]);
                rig.ep().reply(service, token, payload, 2).unwrap();
                rig.collect(index + 1, request);
                assert_eq!(
                    rig.ep().reply(service, token, Payload::EMPTY, 3),
                    Err(Error::Stale)
                );
            }
            assert_eq!(rig.ep().outstanding(), 0);
            assert_eq!(rig.ep().occupancy(), 0);
        }
    }
}

#[test]
fn copy_failure_transactions_keep_request_and_terminal_id_reserved() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let client = rig.ids[1];
    let request = rig.submit(1, 42, b"unchanged").unwrap();
    let before = rig.usages();
    let failed = rig.ep().reserve_receive(service, 1).unwrap();
    let old_token = failed.token;
    rig.ep().finish_receive(&failed, false).unwrap();
    assert_eq!(rig.ep().occupancy(), 1);
    assert_eq!(rig.usages(), before);
    assert_eq!(rig.ep().commit(service, old_token, 1), Err(Error::Stale));
    drop(failed);
    let (token, payload) = rig.receive();
    assert_eq!(token, old_token);
    assert_eq!(payload.bytes(), b"unchanged");
    rig.ep()
        .reply(service, token, Payload::copy(b"result").unwrap(), 2)
        .unwrap();
    let before = rig.usages();
    let failed = rig.ep().reserve_collect(client, request).unwrap();
    rig.ep().finish_collect(&failed, false).unwrap();
    assert_eq!(rig.usages(), before);
    assert_eq!(rig.submit(1, 42, b"duplicate"), Err(Error::Busy));
    drop(failed);
    assert_eq!(rig.collect(1, request).1.bytes(), b"result");
    let replacement = rig.submit(1, 42, b"new").unwrap();
    assert_ne!(request, replacement);
    assert_eq!(rig.ep().outcome(client, request), Err(Error::Stale));
    assert_eq!(rig.ep().commit(service, old_token, 3), Err(Error::Stale));
}

#[test]
fn cancellation_during_copy_never_commits_a_cancelled_token() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let caller = rig.ids[1];
    let request = rig.submit(1, 7, b"copying").unwrap();
    let delivery = rig.ep().reserve_receive(service, 1).unwrap();
    assert_eq!(
        rig.ep().cancel(caller, request, 2),
        Ok(Outcome::CancelledBeforeEffect)
    );
    assert_eq!(
        rig.ep().finish_receive(&delivery, true),
        Err(Error::AlreadyTerminal)
    );
    assert_eq!(
        rig.ep().commit(service, delivery.token, 3),
        Err(Error::Stale)
    );
    assert_eq!(rig.collect(1, request).0, Outcome::CancelledBeforeEffect);
    let new_request = rig.submit(1, 7, b"replacement").unwrap();
    assert_ne!(new_request, request);
    assert_eq!(rig.ep().finish_receive(&delivery, false), Err(Error::Stale));
    drop(delivery);
}

#[test]
fn cancel_head_and_nonhead_preserves_linearized_fifo() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let requests: Vec<_> = (1..=4)
        .map(|i| rig.submit(i, i as u64, &[i as u8]).unwrap())
        .collect();
    let head = rig.ids[1];
    let third = rig.ids[3];
    rig.ep().cancel(head, requests[0], 2).unwrap();
    rig.ep().cancel(third, requests[2], 2).unwrap();
    for i in [2, 4] {
        let (token, payload) = rig.receive();
        assert_eq!(payload.bytes(), &[i as u8]);
        rig.ep().reply(service, token, payload, 3).unwrap();
    }
    assert_eq!(rig.ep().occupancy(), 0);
    for (index, request) in requests.into_iter().enumerate() {
        let expected = if index == 0 || index == 2 {
            Outcome::CancelledBeforeEffect
        } else {
            Outcome::Completed
        };
        assert_eq!(rig.collect(index + 1, request).0, expected);
    }
}

#[test]
fn terminal_arbiter_commit_cancel_deadline_death_and_second_attempt() {
    for committed in [false, true] {
        for cause in 0..3 {
            let mut rig = Rig::new(4);
            let service = rig.ids[0];
            let client = rig.ids[1];
            let request = rig.submit(1, 1, b"work").unwrap();
            let (token, _) = rig.receive();
            if committed {
                rig.ep().commit(service, token, 2).unwrap();
            }
            match cause {
                0 => {
                    rig.ep().cancel(client, request, 3).unwrap();
                }
                1 => rig.ep().expire(1000),
                2 => rig.ep().process_death(service),
                _ => unreachable!(),
            }
            let expected = if committed {
                Outcome::EffectUnknown
            } else if cause == 1 {
                Outcome::ExpiredBeforeEffect
            } else {
                Outcome::CancelledBeforeEffect
            };
            assert_eq!(rig.ep().outcome(client, request), Ok(Some(expected)));
            assert_eq!(
                rig.ep()
                    .reply(service, token, Payload::copy(b"late").unwrap(), 1001),
                Err(Error::Stale)
            );
            assert_eq!(
                rig.ep().cancel(client, request, 1001),
                Err(Error::AlreadyTerminal)
            );
            rig.ep().expire(2000);
            rig.ep().process_death(service);
            assert_eq!(rig.collect(1, request), (expected, Payload::EMPTY));
            assert_eq!(rig.usages()[1].3, 0);
            assert_eq!(rig.usages()[0].2, 0);
            assert_eq!(rig.usages()[0].3, 0);
        }
    }
}

#[test]
fn readiness_absolute_deadline_admission_boundary_preserves_rejected_state() {
    // The readiness fixture selects CLOCK + frequency / 8. Explicit admission
    // times test that contract, not the cause of any guest execution delay.
    let frequency = 62_500_000_u64;
    let sampled_at = 1_078_757_950_u64;
    let deadline = sampled_at + frequency / 8;
    for admitted_at in [deadline - 1, deadline, deadline + 1] {
        let mut rig = Rig::new(1);
        let service = rig.ids[0];
        let client = rig.ids[1];
        let consumer = rig.domains[1].reference();
        let grant = rig.ep().reference().try_clone().unwrap();
        let waiter = WaitKey::new(service).unwrap();
        assert_eq!(rig.ep().wait_readable(service, waiter), Ok(false));
        let before = rig.usages();
        let references = grant.references();
        let result = rig.ep().submit(
            &grant,
            &consumer,
            1,
            deadline,
            Payload::copy(b"ready").unwrap(),
            admitted_at,
        );
        if admitted_at < deadline {
            let request = result.unwrap();
            assert_eq!(rig.ep().occupancy(), 1);
            assert_eq!(rig.ep().outstanding(), 1);
            assert_eq!(rig.ep().next_deadline(), Some(deadline));
            assert_eq!(rig.ep().pending_wakes().count(), 1);
            assert_eq!(rig.usages()[0].2, before[0].2 + 1);
            assert_eq!(rig.usages()[0].3, before[0].3 + 1);
            assert_eq!(rig.usages()[1].3, before[1].3 + 1);
            assert_eq!(
                rig.ep().cancel(client, request, admitted_at),
                Ok(Outcome::CancelledBeforeEffect)
            );
            assert_eq!(rig.collect(1, request).0, Outcome::CancelledBeforeEffect);
            assert_eq!(rig.usages(), before);
        } else {
            assert_eq!(result, Err(Error::Expired));
            assert_eq!(rig.usages(), before);
            assert_eq!(grant.references(), references);
            assert_eq!(rig.ep().occupancy(), 0);
            assert_eq!(rig.ep().outstanding(), 0);
            assert_eq!(rig.ep().next_deadline(), None);
            assert_eq!(rig.ep().waiter_count(), 1);
            assert_eq!(rig.ep().pending_wakes().count(), 0);
        }
    }
}

#[test]
fn deadline_before_admission_and_at_commit_or_reply_boundary() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let client = rig.ids[1];
    let consumer = rig.domains[1].reference();
    let grant = rig.ep().reference().try_clone().unwrap();
    let before = rig.usages();
    for deadline in [0, 1] {
        assert_eq!(
            rig.ep()
                .submit(&grant, &consumer, 1, deadline, Payload::EMPTY, 1),
            Err(Error::Expired)
        );
    }
    assert_eq!(before, rig.usages());
    let request = rig.submit(1, 1, b"deadline").unwrap();
    assert_eq!(rig.ep().next_deadline(), Some(1000));
    let (token, _) = rig.receive();
    assert_eq!(
        rig.ep().commit(service, token, 1000),
        Err(Error::AlreadyTerminal)
    );
    assert_eq!(
        rig.ep().outcome(client, request),
        Ok(Some(Outcome::ExpiredBeforeEffect))
    );
    rig.collect(1, request);
    let request = rig.submit(1, 1, b"next").unwrap();
    let (token, _) = rig.receive();
    rig.ep().commit(service, token, 999).unwrap();
    assert_eq!(
        rig.ep().reply(service, token, Payload::EMPTY, 1000),
        Err(Error::AlreadyTerminal)
    );
    assert_eq!(rig.collect(1, request).0, Outcome::EffectUnknown);
    assert_eq!(rig.ep().next_deadline(), None);
}

#[test]
fn process_death_before_after_commit_and_retained_domain_lifetime() {
    for phase in 0..3 {
        let mut rig = Rig::new(4);
        let service = rig.ids[0];
        let client = rig.ids[1];
        let request = rig.submit(1, 1, b"retained").unwrap();
        let token = if phase > 0 {
            Some(rig.receive().0)
        } else {
            None
        };
        if phase == 2 {
            rig.ep().commit(service, token.unwrap(), 2).unwrap();
        }
        rig.domains[1].close();
        rig.ep().process_death(client);
        assert_eq!(
            rig.ep().outcome(client, request),
            Err(if phase == 2 {
                Error::Denied
            } else {
                Error::Stale
            })
        );
        if phase == 2 {
            assert_eq!(rig.usages()[1].3, 1);
            rig.ep()
                .reply(service, token.unwrap(), Payload::EMPTY, 3)
                .unwrap();
        }
        assert_eq!(rig.usages()[1].3, 0);
        assert_eq!(rig.ep().outstanding(), 0);
    }
}

#[test]
fn exact_waiter_registration_ready_ack_and_shutdown_quiescence() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let client = rig.ids[1];
    let foreign = rig.ids[2];
    let readable = WaitKey::new(service).unwrap();
    assert_eq!(
        rig.ep()
            .wait_readable(foreign, WaitKey::new(foreign).unwrap()),
        Err(Error::Denied)
    );
    assert_eq!(rig.ep().wait_readable(service, readable), Ok(false));
    assert_eq!(
        rig.ep()
            .wait_readable(service, WaitKey::new(service).unwrap()),
        Err(Error::Busy)
    );
    let request = rig.submit(1, 7, b"awake").unwrap();
    assert_eq!(rig.ep().pending_wakes().count(), 1);
    assert_eq!(rig.ep().pending_wakes().next().unwrap().key, readable);
    assert_eq!(
        rig.ep().acknowledge_wake(WaitKey::new(service).unwrap()),
        Err(Error::Stale)
    );
    rig.ep().acknowledge_wake(readable).unwrap();
    assert_eq!(rig.ep().acknowledge_wake(readable), Err(Error::Stale));
    let terminal = WaitKey::new(client).unwrap();
    assert_eq!(rig.ep().wait_terminal(client, request, terminal), Ok(false));
    assert_eq!(
        rig.ep()
            .wait_terminal(foreign, request, WaitKey::new(foreign).unwrap()),
        Err(Error::Denied)
    );
    rig.ep().expire(1000);
    let wake = rig.ep().pending_wakes().next().unwrap();
    assert_eq!(wake.key, terminal);
    assert_eq!(
        wake.target,
        WaitTarget::Terminal(rig.ep().reference().id(), request)
    );
    assert_eq!(rig.collect(1, request).0, Outcome::ExpiredBeforeEffect);
    rig.ep().shutdown();
    assert!(!rig.ep().reclaimable());
    rig.ep().acknowledge_wake(terminal).unwrap();
    assert!(rig.ep().reclaimable());
}

#[test]
fn signal_before_registration_and_waiter_death_need_no_polling_semantics() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let client = rig.ids[1];
    let request = rig.submit(1, 1, b"ready").unwrap();
    assert_eq!(
        rig.ep()
            .wait_readable(service, WaitKey::new(service).unwrap()),
        Ok(true)
    );
    let (token, _) = rig.receive();
    rig.ep().commit(service, token, 2).unwrap();
    let key = WaitKey::new(client).unwrap();
    assert_eq!(rig.ep().wait_terminal(client, request, key), Ok(false));
    rig.ep().process_death(client);
    assert_eq!(rig.ep().waiter_count(), 0);
    rig.ep().reply(service, token, Payload::EMPTY, 3).unwrap();
    assert_eq!(rig.ep().outstanding(), 0);
    assert_eq!(rig.ep().pending_wakes().count(), 0);
    let key = WaitKey::new(service).unwrap();
    assert_eq!(rig.ep().wait_readable(service, key), Ok(false));
    rig.ep().shutdown();
    assert_eq!(rig.ep().pending_wakes().next().unwrap().key, key);
}

#[test]
fn caller_local_endpoint_grants_receiver_binding_transfer_and_revoke() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let client = rig.ids[1];
    let recipient = rig.ids[2];
    let mut sender = Namespace::<8>::new();
    sender
        .bind_domain(client, rig.domains[1].reference())
        .unwrap();
    let mut receiver = Namespace::<8>::new();
    receiver
        .bind_domain(service, rig.domains[0].reference())
        .unwrap();
    let mut delegate = Namespace::<8>::new();
    delegate
        .bind_domain(recipient, rig.domains[2].reference())
        .unwrap();
    let handle = sender
        .create_endpoint_sender(
            client,
            rig.ep().reference().try_clone().unwrap(),
            Rights::ALL,
            |_| Ok::<_, ()>(()),
        )
        .unwrap();
    let receive = receiver
        .create_endpoint_receiver(service, rig.ep().reference().try_clone().unwrap(), |_| {
            Ok::<_, ()>(())
        })
        .unwrap();
    assert!(sender.lookup(client, handle, Kind::Event).is_err());
    assert!(sender.lookup(recipient, handle, Kind::Endpoint).is_err());
    assert!(
        sender
            .lookup(client, Handle::decode(0), Kind::Endpoint)
            .is_err()
    );
    assert!(
        sender
            .lookup(client, handle, Kind::Endpoint)
            .unwrap()
            .endpoint_receiver(client)
            .is_err()
    );
    assert!(
        receiver
            .lookup(service, receive, Kind::Endpoint)
            .unwrap()
            .endpoint_sender()
            .is_err()
    );
    assert!(
        receiver
            .lookup(service, receive, Kind::Endpoint)
            .unwrap()
            .endpoint_receiver(service)
            .is_ok()
    );
    assert!(
        receiver
            .transfer(service, receive, &mut delegate, recipient, Rights::NONE)
            .is_err()
    );
    let delegated = sender
        .transfer(client, handle, &mut delegate, recipient, Rights::SEND)
        .unwrap();
    assert!(
        delegate
            .transfer(recipient, delegated, &mut receiver, service, Rights::ALL)
            .is_err()
    );
    let grant = sender
        .lookup(client, handle, Kind::Endpoint)
        .unwrap()
        .endpoint_sender()
        .unwrap()
        .try_clone()
        .unwrap();
    let consumer = rig.domains[1].reference();
    let request = rig
        .ep()
        .submit(
            &grant,
            &consumer,
            17,
            1000,
            Payload::copy(b"accepted").unwrap(),
            1,
        )
        .unwrap();
    sender.revoke(client, handle).unwrap();
    sender.close(client, handle).unwrap();
    assert!(sender.lookup(client, handle, Kind::Endpoint).is_err());
    let alias = delegate
        .lookup(recipient, delegated, Kind::Endpoint)
        .unwrap()
        .endpoint_sender()
        .unwrap()
        .try_clone()
        .unwrap();
    let other = rig.domains[2].reference();
    let before = rig.usages();
    assert_eq!(
        rig.ep().submit(&alias, &other, 18, 1000, Payload::EMPTY, 1),
        Err(Error::Denied)
    );
    assert_eq!(before, rig.usages());
    let (token, payload) = rig.receive();
    assert_eq!(
        rig.ep().reply(recipient, token, Payload::EMPTY, 2),
        Err(Error::Denied)
    );
    rig.ep().reply(service, token, payload, 2).unwrap();
    assert_eq!(rig.collect(1, request).1.bytes(), b"accepted");
}

#[test]
fn receiver_and_endpoint_quota_rejections_are_transactional() {
    let mut rig = Rig::new(4);
    let client = rig.ids[1];
    let service = rig.ids[0];
    let mut client_table = Namespace::<1>::new();
    client_table
        .bind_domain(client, rig.domains[1].reference())
        .unwrap();
    let before = rig.usages();
    assert!(
        client_table
            .create_endpoint_receiver(client, rig.ep().reference().try_clone().unwrap(), |_| Ok::<
                _,
                (),
            >(
                ()
            ))
            .is_err()
    );
    assert_eq!(before, rig.usages());
    assert!(matches!(
        Endpoint::<4, 8>::create(rig.domains[1].reference(), 0, 1),
        Err(Error::Exhausted)
    ));
    assert!(matches!(
        Endpoint::<4, 8>::create(rig.domains[0].reference(), 0, 1),
        Err(Error::Exhausted)
    ));
    assert_eq!(rig.domains[0].reference().endpoint_usage(), 1);
    assert!(matches!(
        Endpoint::<4, 8>::create(rig.domains[0].reference(), 0, 5),
        Err(Error::Invalid)
    ));
    let none = client_table
        .create_endpoint_sender(
            client,
            rig.ep().reference().try_clone().unwrap(),
            Rights::NONE,
            |_| Ok::<_, ()>(()),
        )
        .unwrap();
    assert!(
        client_table
            .lookup(client, none, Kind::Endpoint)
            .unwrap()
            .endpoint_sender()
            .is_err()
    );
    let service_before = rig.domains[0].reference().endpoint_usage();
    rig.ep().process_death(service);
    assert_eq!(rig.domains[0].reference().endpoint_usage(), service_before);
}

#[test]
fn endpoint_and_token_generation_reuse_never_rebinds_stale_identity() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let request = rig.submit(1, 1, b"old").unwrap();
    let (token, _) = rig.receive();
    rig.ep().reply(service, token, Payload::EMPTY, 2).unwrap();
    rig.collect(1, request);
    let old = rig.ep().reference().id();
    let retained = rig.ep().reference().try_clone().unwrap();
    rig.ep().shutdown();
    assert!(!rig.ep().reclaimable());
    drop(retained);
    assert!(rig.ep().reclaimable());
    drop(rig.endpoint.take().unwrap());
    rig.endpoint = Some(Endpoint::create(rig.domains[0].reference(), 0, 4).unwrap());
    let replacement = rig.ep().reference().id();
    assert_eq!(replacement.slot(), old.slot());
    assert!(replacement.generation() > old.generation());
    let new_request = rig.submit(1, 1, b"new").unwrap();
    assert_ne!(request, new_request);
    assert_eq!(rig.ep().commit(service, token, 3), Err(Error::Stale));
    assert_eq!(
        rig.ep().reply(service, token, Payload::EMPTY, 3),
        Err(Error::Stale)
    );
    let guessed = ServiceToken::decode(new_request.receipt());
    assert_eq!(
        rig.ep().reply(service, guessed, Payload::EMPTY, 3),
        Err(Error::Stale)
    );
}

#[test]
fn closed_service_admission_and_zero_client_quota_leave_no_phantom_work() {
    let mut rig = Rig::new(4);
    let caller = rig.ids[1];
    let (zero, _pages) = Owner::new(
        caller,
        Limits {
            memory_pages: 1,
            handles: 1,
            queue: 0,
            requests: 0,
            endpoints: 0,
        },
        1,
    )
    .unwrap();
    let grant = rig.ep().reference().try_clone().unwrap();
    let before = rig.usages();
    assert_eq!(
        rig.ep()
            .submit(&grant, &zero.reference(), 1, 1000, Payload::EMPTY, 1),
        Err(Error::Exhausted)
    );
    assert_eq!(rig.usages(), before);
    assert_eq!(rig.ep().outstanding(), 0);
    rig.domains[0].close();
    assert_eq!(rig.submit(1, 1, b"closed"), Err(Error::Denied));
    assert_eq!(rig.ep().occupancy(), 0);
}

#[test]
fn endpoint_references_exhaust_and_recover_without_silent_wrap() {
    let mut rig = Rig::new(4);
    let mut retained = Vec::new();
    for _ in 0..1023 {
        retained.push(rig.ep().reference().try_clone().unwrap());
    }
    assert!(matches!(
        rig.ep().reference().try_clone(),
        Err(Error::Exhausted)
    ));
    assert_eq!(rig.ep().reference().references(), 1024);
    drop(retained);
    assert_eq!(rig.ep().reference().references(), 1);
    let sequence = core::sync::atomic::AtomicU64::new(u64::MAX - 1);
    assert_eq!(super::identity::next(&sequence), Ok(u64::MAX));
    assert_eq!(super::identity::next(&sequence), Err(Error::Exhausted));
    assert_eq!(
        sequence.load(core::sync::atomic::Ordering::Acquire),
        u64::MAX
    );
}

#[test]
fn real_arbiter_under_concurrent_cancel_reply_and_deadline() {
    let mut rig = Rig::new(4);
    let service = rig.ids[0];
    let client = rig.ids[1];
    for round in 0..64 {
        let request = rig.submit(1, round, b"race").unwrap();
        let (token, _) = rig.receive();
        if round % 2 == 0 {
            rig.ep().commit(service, token, 2).unwrap();
        }
        let endpoint = Mutex::new(rig.endpoint.take().unwrap());
        let results = std::thread::scope(|scope| {
            let reply = scope.spawn(|| {
                endpoint.lock().unwrap().reply(
                    service,
                    token,
                    Payload::copy(b"response").unwrap(),
                    3,
                )
            });
            let cancel = scope.spawn(|| endpoint.lock().unwrap().cancel(client, request, 3));
            let deadline = scope.spawn(|| endpoint.lock().unwrap().expire(1000));
            (
                reply.join().unwrap(),
                cancel.join().unwrap(),
                deadline.join().unwrap(),
            )
        });
        rig.endpoint = Some(endpoint.into_inner().unwrap());
        let outcome = rig.ep().outcome(client, request).unwrap().unwrap();
        assert!(
            results.0.is_ok()
                || results.1.is_ok()
                || outcome == Outcome::ExpiredBeforeEffect
                || outcome == Outcome::EffectUnknown
        );
        let (observed, payload) = rig.collect(1, request);
        assert_eq!(observed, outcome);
        if outcome == Outcome::Completed {
            assert_eq!(payload.bytes(), b"response");
        } else {
            assert!(payload.is_empty());
        }
        assert_eq!(rig.usages()[0].2, 0);
        assert_eq!(rig.usages()[0].3, 0);
        assert_eq!(rig.usages()[1].3, 0);
    }
}

fn frame(operation: wire::Operation, payload: &[u8]) -> Vec<u8> {
    let mut bytes = std::vec![0; wire::HEADER + payload.len()];
    bytes[..2].copy_from_slice(&wire::VERSION.to_le_bytes());
    bytes[2..4].copy_from_slice(&operation.encode().to_le_bytes());
    let length = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&length.to_le_bytes());
    bytes[8..16].copy_from_slice(&0x100_u64.to_le_bytes());
    bytes[32..36].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes[wire::HEADER..].copy_from_slice(payload);
    bytes
}
#[test]
fn wire_rejects_malformed_unknown_overflow_and_unused_fields() {
    let valid = frame(wire::Operation::Submit, &[1; 256]);
    assert_eq!(wire::Input::decode(&valid).unwrap().payload.len(), 256);
    for length in 0..wire::HEADER {
        assert!(wire::Input::decode(&valid[..length]).is_err());
    }
    for (offset, bytes) in [
        (0, 0_u32.to_le_bytes()),
        (0, 99_u32.to_le_bytes()),
        (4, u32::MAX.to_le_bytes()),
        (32, u32::MAX.to_le_bytes()),
        (36, 1_u32.to_le_bytes()),
    ] {
        let mut bad = valid.clone();
        bad[offset..offset + 4].copy_from_slice(&bytes);
        assert!(wire::Input::decode(&bad).is_err());
    }
    assert!(wire::Input::decode(&frame(wire::Operation::Submit, &[1; 257])).is_err());
    for operation in [
        wire::Operation::Receive,
        wire::Operation::Commit,
        wire::Operation::Wait,
        wire::Operation::Collect,
        wire::Operation::Cancel,
        wire::Operation::Shutdown,
        wire::Operation::Abandon,
    ] {
        let mut bad = frame(operation, &[]);
        bad[24] = 1;
        assert!(wire::Input::decode(&bad).is_err());
        assert!(wire::Input::decode(&frame(operation, &[1])).is_err());
        if operation != wire::Operation::Commit {
            let mut bad = frame(operation, &[]);
            bad[16] = 1;
            assert!(wire::Input::decode(&bad).is_err());
        }
    }
    let mut state = 7_u64;
    for _ in 0..1024 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut bad = valid.clone();
        bad[36..40].copy_from_slice(&((state as u32) | 1).to_le_bytes());
        assert!(wire::Input::decode(&bad).is_err());
    }
}
