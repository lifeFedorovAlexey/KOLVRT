//! UNIT inputs for the single production IPC protocol; no service implementation.
use kernel_core::{
    domain::{Limits, Owner},
    ipc::{Endpoint, Error, Outcome, Payload, ServiceToken, mailbox::Mailbox},
    process::Table,
};
fn limits() -> Limits {
    Limits {
        memory_pages: 1,
        handles: 4,
        queue: 4,
        requests: 4,
        endpoints: 1,
    }
}
pub(super) fn exercise(mut report: impl FnMut(&str, bool)) {
    let mailbox = Mailbox::new();
    let invalid = mailbox.publish(0) == Err(Error::Invalid) && mailbox.quiescent();
    let published = mailbox.publish(1) == Ok(true) && mailbox.pending() == Some(1);
    let retained = mailbox.publish(2) == Err(Error::Busy)
        && mailbox.consume(2) == Err(Error::Stale)
        && mailbox.pending() == Some(1);
    mailbox.consume(1).unwrap();
    let ack_retained = mailbox.finish_acknowledgement(2) == Err(Error::Stale)
        && mailbox.acknowledged(1)
        && !mailbox.acknowledged(2);
    mailbox.finish_acknowledgement(1).unwrap();
    report(
        "ipc_mailbox_publication_and_generation_guards",
        invalid && published && retained && ack_retained && mailbox.quiescent(),
    );

    let mut identities = Table::<2>::new();
    let service = identities.reserve(0..1).unwrap();
    let client = identities.reserve(1..2).unwrap();
    let (service_owner, _service_memory) = Owner::new(service, limits(), 1).unwrap();
    let (client_owner, _client_memory) = Owner::new(client, limits(), 1).unwrap();
    let service_domain = service_owner.reference();
    let client_domain = client_owner.reference();
    let mut endpoint = Endpoint::<1, 1>::create(service_owner.reference(), 0, 1).unwrap();
    let grant = endpoint.reference().try_clone().unwrap();
    let request = endpoint
        .submit(&grant, &client_domain, 1, 100, Payload::EMPTY, 1)
        .unwrap();
    let delivery = endpoint.reserve_receive(service, 2).unwrap();
    endpoint.finish_receive(&delivery, true).unwrap();
    let before = (service_domain.usage(), client_domain.usage());
    let invalid_caller = endpoint.commit(client, delivery.token, 3) == Err(Error::Denied)
        && (service_domain.usage(), client_domain.usage()) == before;
    let invalid_token = endpoint.commit(service, ServiceToken::decode(0), 3) == Err(Error::Stale)
        && (service_domain.usage(), client_domain.usage()) == before;
    report(
        "ipc_service_token_invalid_caller_rejected",
        invalid_caller && invalid_token && endpoint.outcome(client, request) == Ok(None),
    );
    endpoint.commit(service, delivery.token, 3).unwrap();
    let first = endpoint
        .reply(service, delivery.token, Payload::EMPTY, 4)
        .unwrap();
    let before = (service_domain.usage(), client_domain.usage());
    let rejected = endpoint.cancel(client, request, 5) == Err(Error::AlreadyTerminal)
        && endpoint.reply(service, delivery.token, Payload::EMPTY, 5) == Err(Error::Stale)
        && endpoint.outcome(client, request) == Ok(Some(Outcome::Completed))
        && (service_domain.usage(), client_domain.usage()) == before;
    let collection = endpoint.reserve_collect(client, request).unwrap();
    endpoint.finish_collect(&collection, true).unwrap();
    drop(collection);
    drop(delivery);
    endpoint.shutdown();
    drop(grant);
    let drained = endpoint.reclaimable()
        && endpoint.outstanding() == 0
        && service_domain.usage().3 == 0
        && client_domain.usage().3 == 0;
    drop(endpoint);
    report(
        "ipc_duplicate_terminal_retains_outcome_and_charges",
        first == Outcome::Completed && rejected && drained,
    );
}
#[cfg(feature = "ipc-teardown-input")]
pub(super) fn retained_endpoint_drop() {
    let mut identities = Table::<1>::new();
    let service = identities.reserve(0..1).unwrap();
    let (owner, _memory) = Owner::new(service, limits(), 1).unwrap();
    let mut endpoint = Endpoint::<1, 1>::create(owner.reference(), 0, 1).unwrap();
    let _retained = endpoint.reference().try_clone().unwrap();
    endpoint.shutdown();
    drop(endpoint);
    panic!("retained endpoint drop was accepted");
}
