//! Bounded concrete IPC state. The kernel serializes each Endpoint separately,
//! outside scheduler storage scopes. The arbiter never invokes external code.
mod identity;
pub mod mailbox;
pub mod wire;
use crate::{
    domain::{Charge, ChargeKind, Reference as Domain},
    process::ProcessId,
};
pub use identity::{ENDPOINTS, EndpointId, Reference};

pub const MAX_PAYLOAD: usize = 256;
pub const MAX_REQUESTS: usize = 8;
pub const MAX_WAITERS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Denied,
    Exhausted,
    Unsupported,
    Stale,
    Busy,
    AlreadyTerminal,
    Expired,
    ChargeAlreadyReleased,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    CancelledBeforeEffect,
    EffectUnknown,
    ExpiredBeforeEffect,
}
impl Outcome {
    pub fn encode(self) -> u16 {
        match self {
            Self::Completed => 0,
            Self::CancelledBeforeEffect => 4,
            Self::EffectUnknown => 5,
            Self::ExpiredBeforeEffect => 6,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payload {
    bytes: [u8; MAX_PAYLOAD],
    length: usize,
}
impl Payload {
    pub const EMPTY: Self = Self {
        bytes: [0; MAX_PAYLOAD],
        length: 0,
    };
    pub fn copy(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_PAYLOAD {
            return Err(Error::Invalid);
        }
        let mut result = Self::EMPTY;
        result.bytes[..bytes.len()].copy_from_slice(bytes);
        result.length = bytes.len();
        Ok(result)
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestId {
    slot: usize,
    generation: u64,
}
impl RequestId {
    pub fn receipt(self) -> u64 {
        self.generation
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceToken(u64);
impl ServiceToken {
    pub fn decode(value: u64) -> Self {
        Self(value)
    }
    pub fn encode(self) -> u64 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitKey {
    process: ProcessId,
    sequence: u64,
}
impl WaitKey {
    pub fn new(process: ProcessId) -> Result<Self, Error> {
        Ok(Self {
            process,
            sequence: identity::next_wait()?,
        })
    }
    pub fn process(self) -> ProcessId {
        self.process
    }
    pub fn sequence(self) -> u64 {
        self.sequence
    }
    #[cfg(feature = "ipc-wake-generation-negative")]
    pub fn with_generation(self, generation: u64) -> Self {
        Self {
            process: self.process.with_generation_for_test(generation),
            sequence: self.sequence,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitTarget {
    Readable(EndpointId),
    Terminal(EndpointId, RequestId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wake {
    pub key: WaitKey,
    pub target: WaitTarget,
}
struct WaitSlot {
    wake: Wake,
    ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Queued,
    Delivered,
    Committed,
    Terminal(Outcome),
}
enum Cause<'a> {
    Complete(&'a Payload),
    Cancel,
    Deadline,
    ServiceDeath,
}
struct Request {
    identity: RequestId,
    consumer: ProcessId,
    client_id: u64,
    deadline: u64,
    payload: Payload,
    response: Payload,
    phase: Phase,
    alive: bool,
    receive_reserved: bool,
    collect_reserved: bool,
    wait: Option<usize>,
    _endpoint: Reference,
    _consumer: Charge,
    queue: Option<Charge>,
    active: Option<Charge>,
}
/// One linear active charge belongs to every nonterminal accepted request.
/// A second release attempt fails before any domain counter can be touched.
fn release_active(request: &mut Request) -> Result<(), Error> {
    let charge = request.active.take().ok_or(Error::ChargeAlreadyReleased)?;
    drop(charge);
    Ok(())
}

/// Kernel-owned copied reservation; no borrow into endpoint or user memory escapes.
pub struct Delivery {
    endpoint: Reference,
    identity: RequestId,
    pub token: ServiceToken,
    pub client_id: u64,
    pub deadline: u64,
    pub payload: Payload,
}
impl Delivery {
    pub fn endpoint(&self) -> &Reference {
        &self.endpoint
    }
}
pub struct Collection {
    endpoint: Reference,
    identity: RequestId,
    pub client_id: u64,
    pub outcome: Outcome,
    pub payload: Payload,
}
impl Collection {
    pub fn endpoint(&self) -> &Reference {
        &self.endpoint
    }
}

/// Q is queue capacity; R independently bounds queued/delivered/retained results.
/// Every operation needs an exclusive endpoint permit, not a global IPC permit.
pub struct Endpoint<const Q: usize = 4, const R: usize = MAX_REQUESTS> {
    reference: Reference,
    service: Domain,
    _ownership: Charge,
    capacity: usize,
    requests: [Option<Request>; R],
    queue: [Option<RequestId>; Q],
    queued: usize,
    receive_wait: Option<usize>,
    waits: [Option<WaitSlot>; MAX_WAITERS],
}
fn charge(domain: &Domain, kind: ChargeKind) -> Result<Charge, Error> {
    domain.charge(kind).map_err(|e| match e {
        crate::domain::Error::Exhausted => Error::Exhausted,
        _ => Error::Denied,
    })
}
impl<const Q: usize, const R: usize> Endpoint<Q, R> {
    pub fn create(service: Domain, cpu: usize, capacity: usize) -> Result<Self, Error> {
        if Q == 0 || R == 0 || R > MAX_REQUESTS || capacity == 0 || capacity > Q {
            return Err(Error::Invalid);
        }
        service
            .validate(service.owner())
            .map_err(|_| Error::Denied)?;
        let ownership = charge(&service, ChargeKind::Endpoint)?;
        let reference = Reference::create(service.owner(), cpu)?;
        Ok(Self {
            reference,
            service,
            _ownership: ownership,
            capacity,
            requests: [const { None }; R],
            queue: [None; Q],
            queued: 0,
            receive_wait: None,
            waits: [const { None }; MAX_WAITERS],
        })
    }
    pub fn reference(&self) -> &Reference {
        &self.reference
    }
    pub fn occupancy(&self) -> usize {
        self.queued
    }
    pub fn outstanding(&self) -> usize {
        self.requests.iter().filter(|r| r.is_some()).count()
    }
    pub fn waiter_count(&self) -> usize {
        self.waits.iter().filter(|w| w.is_some()).count()
    }
    /// Accepted receipt lookup requires the exact live requester identity. The
    /// receipt is independent of a subsequently closed or revoked SEND handle.
    pub fn receipt(&self, caller: ProcessId, value: u64) -> Result<RequestId, Error> {
        self.requests
            .iter()
            .flatten()
            .find(|r| {
                (cfg!(feature = "ipc-request-generation-negative") || r.identity.receipt() == value)
                    && r.consumer == caller
                    && r.alive
            })
            .map(|r| r.identity)
            .ok_or(Error::Stale)
    }
    pub fn reclaimable(&self) -> bool {
        (cfg!(feature = "ipc-teardown-negative") && self.reference.closed())
            || (self.reference.closed()
                && self.outstanding() == 0
                && self.waiter_count() == 0
                && self.reference.references() == 1)
    }
    fn request(&self, id: RequestId) -> Result<&Request, Error> {
        self.requests
            .get(id.slot)
            .and_then(Option::as_ref)
            .filter(|r| r.identity == id)
            .ok_or(Error::Stale)
    }
    fn request_mut(&mut self, id: RequestId) -> Result<&mut Request, Error> {
        self.requests
            .get_mut(id.slot)
            .and_then(Option::as_mut)
            .filter(|r| r.identity == id)
            .ok_or(Error::Stale)
    }
    fn consumer(&self, caller: ProcessId, id: RequestId) -> Result<&Request, Error> {
        let r = self.request(id)?;
        if r.consumer != caller || !r.alive {
            return Err(Error::Denied);
        }
        Ok(r)
    }
    fn service(&self, caller: ProcessId) -> Result<(), Error> {
        if caller != self.reference.service() {
            return Err(Error::Denied);
        }
        Ok(())
    }
    fn token(&self, caller: ProcessId, token: ServiceToken) -> Result<RequestId, Error> {
        self.service(caller)?;
        self.requests
            .iter()
            .flatten()
            .find(|r| {
                r.identity.generation == token.0
                    && matches!(r.phase, Phase::Delivered | Phase::Committed)
            })
            .map(|r| r.identity)
            .ok_or(Error::Stale)
    }
    pub fn submit(
        &mut self,
        grant: &Reference,
        consumer: &Domain,
        client_id: u64,
        deadline: u64,
        payload: Payload,
        now: u64,
    ) -> Result<RequestId, Error> {
        if grant.id() != self.reference.id() || !grant.open() {
            return Err(Error::Denied);
        }
        consumer
            .validate(consumer.owner())
            .map_err(|_| Error::Denied)?;
        self.service
            .validate(self.reference.service())
            .map_err(|_| Error::Denied)?;
        if deadline <= now {
            return Err(Error::Expired);
        }
        if !cfg!(feature = "ipc-id-reuse-negative")
            && self
                .requests
                .iter()
                .flatten()
                .any(|r| r.consumer == consumer.owner() && r.client_id == client_id)
        {
            return Err(Error::Busy);
        }
        if self.queued == self.capacity
            && !(cfg!(feature = "ipc-capacity-negative") && self.capacity < Q)
        {
            return Err(Error::Exhausted);
        }
        let slot = self
            .requests
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Exhausted)?;
        let consumer_charge = charge(consumer, ChargeKind::Request)?;
        let queue_charge = charge(&self.service, ChargeKind::Queue)?;
        let active_charge = charge(&self.service, ChargeKind::Request)?;
        let retained = grant.try_clone()?;
        let identity = RequestId {
            slot,
            generation: identity::next_request()?,
        };
        grant.admit()?;
        self.requests[slot] = Some(Request {
            identity,
            consumer: consumer.owner(),
            client_id,
            deadline,
            payload,
            response: Payload::EMPTY,
            phase: Phase::Queued,
            alive: true,
            receive_reserved: false,
            collect_reserved: false,
            wait: None,
            _endpoint: retained,
            _consumer: consumer_charge,
            queue: Some(queue_charge),
            active: Some(active_charge),
        });
        self.queue[self.queued] = Some(identity);
        self.queued += 1;
        if let Some(wait) = self.receive_wait.take() {
            self.notify(wait);
        }
        Ok(identity)
    }
    pub fn reserve_receive(&mut self, caller: ProcessId, now: u64) -> Result<Delivery, Error> {
        self.service(caller)?;
        self.expire(now);
        let head = if cfg!(feature = "ipc-fifo-negative") && self.queued > 1 {
            self.queued - 1
        } else {
            0
        };
        let id = self.queue[head].ok_or(if self.reference.closed() {
            Error::Denied
        } else {
            Error::Busy
        })?;
        let endpoint = self.reference.try_clone()?;
        let r = self.request_mut(id)?;
        if r.receive_reserved {
            return Err(Error::Busy);
        }
        r.receive_reserved = true;
        Ok(Delivery {
            endpoint,
            identity: id,
            token: ServiceToken(if cfg!(feature = "ipc-service-token-negative") {
                id.generation.wrapping_add(1)
            } else {
                id.generation
            }),
            client_id: r.client_id,
            deadline: r.deadline,
            payload: r.payload.clone(),
        })
    }
    pub fn finish_receive(&mut self, delivery: &Delivery, copied: bool) -> Result<(), Error> {
        if delivery.endpoint.id() != self.reference.id() {
            return Err(Error::Stale);
        }
        let r = self.request_mut(delivery.identity)?;
        if !r.receive_reserved {
            return Err(Error::Stale);
        }
        r.receive_reserved = false;
        if r.phase != Phase::Queued {
            return Err(Error::AlreadyTerminal);
        }
        if copied || cfg!(feature = "ipc-receive-copy-negative") {
            r.phase = Phase::Delivered;
            r.queue = None;
            self.remove_queue(delivery.identity);
        }
        Ok(())
    }
    fn remove_queue(&mut self, id: RequestId) {
        if let Some(index) = self.queue[..self.queued]
            .iter()
            .position(|entry| *entry == Some(id))
        {
            for i in index..self.queued - 1 {
                self.queue[i] = self.queue[i + 1];
            }
            self.queued -= 1;
            self.queue[self.queued] = None;
        }
    }
    fn notify(&mut self, wait: usize) {
        self.waits[wait]
            .as_mut()
            .expect("owned waiter reservation")
            .ready = true;
    }
    /// The ONLY transition to terminal state. Endpoint exclusion linearizes all causes.
    fn terminal(&mut self, id: RequestId, cause: Cause<'_>) -> Result<Outcome, Error> {
        let r = self.request_mut(id)?;
        if !cfg!(feature = "ipc-double-terminal-negative") && matches!(r.phase, Phase::Terminal(_))
        {
            return Err(Error::AlreadyTerminal);
        }
        let committed = r.phase == Phase::Committed;
        let outcome = match cause {
            Cause::Cancel if cfg!(feature = "ipc-cancel-negative") => Outcome::Completed,
            Cause::ServiceDeath if cfg!(feature = "ipc-service-death-negative") => {
                Outcome::Completed
            }
            Cause::Complete(payload) => {
                r.response = payload.clone();
                Outcome::Completed
            }
            Cause::Deadline if !committed => Outcome::ExpiredBeforeEffect,
            Cause::Cancel | Cause::ServiceDeath if !committed => Outcome::CancelledBeforeEffect,
            _ => Outcome::EffectUnknown,
        };
        r.phase = Phase::Terminal(outcome);
        r.queue = None;
        let released = release_active(r);
        #[cfg(feature = "ipc-double-charge-release-negative")]
        let released = released.and_then(|()| release_active(r));
        let wait = r.wait.take();
        let alive = r.alive;
        self.remove_queue(id);
        if let Some(wait) = wait {
            self.notify(wait);
        }
        if !alive {
            self.requests[id.slot] = None;
        }
        released?;
        Ok(outcome)
    }
    fn expire_request(&mut self, id: RequestId, now: u64) -> bool {
        if cfg!(feature = "ipc-deadline-negative") {
            return false;
        }
        if self
            .request(id)
            .is_ok_and(|r| !matches!(r.phase, Phase::Terminal(_)) && r.deadline <= now)
        {
            let _ = self.terminal(id, Cause::Deadline);
            true
        } else {
            false
        }
    }
    pub fn expire(&mut self, now: u64) {
        self.observe_deaths();
        for slot in 0..R {
            if let Some(r) = &self.requests[slot] {
                let id = r.identity;
                self.expire_request(id, now);
            }
        }
    }
    /// Domain closing is published before deferred per-endpoint death drainage.
    /// A service operation must observe that publication itself: a busy earlier
    /// cell may not let uncommitted dead-requester work acquire commitment.
    fn observe_deaths(&mut self) {
        if self.service.closing() && !self.reference.closed() {
            self.shutdown();
        }
        for slot in 0..R {
            let dead = self.requests[slot]
                .as_ref()
                .filter(|request| request.alive && request._consumer.domain().closing())
                .map(|request| (request.consumer, request.identity));
            if let Some((consumer, id)) = dead {
                self.abandon(consumer, id).expect("exact closing requester");
            }
        }
    }
    pub fn next_deadline(&self) -> Option<u64> {
        self.requests
            .iter()
            .flatten()
            .filter(|r| !matches!(r.phase, Phase::Terminal(_)))
            .map(|r| r.deadline)
            .min()
    }
    pub fn commit(
        &mut self,
        caller: ProcessId,
        token: ServiceToken,
        now: u64,
    ) -> Result<(), Error> {
        self.observe_deaths();
        let id = self.token(caller, token)?;
        if self.expire_request(id, now) {
            return Err(Error::AlreadyTerminal);
        }
        let r = self.request_mut(id)?;
        if r.phase != Phase::Delivered {
            return Err(Error::Busy);
        }
        r.phase = Phase::Committed;
        Ok(())
    }
    pub fn reply(
        &mut self,
        caller: ProcessId,
        token: ServiceToken,
        payload: Payload,
        now: u64,
    ) -> Result<Outcome, Error> {
        self.observe_deaths();
        let id = self.token(caller, token)?;
        if self.expire_request(id, now) {
            return Err(Error::AlreadyTerminal);
        }
        self.terminal(id, Cause::Complete(&payload))
    }
    pub fn cancel(&mut self, caller: ProcessId, id: RequestId, now: u64) -> Result<Outcome, Error> {
        self.consumer(caller, id)?;
        if self.expire_request(id, now) {
            return self.outcome(caller, id)?.ok_or(Error::Stale);
        }
        self.terminal(id, Cause::Cancel)
    }
    pub fn outcome(&self, caller: ProcessId, id: RequestId) -> Result<Option<Outcome>, Error> {
        Ok(match self.consumer(caller, id)?.phase {
            Phase::Terminal(outcome) => Some(outcome),
            _ => None,
        })
    }
    pub fn reserve_collect(
        &mut self,
        caller: ProcessId,
        id: RequestId,
    ) -> Result<Collection, Error> {
        let r = self.consumer(caller, id)?;
        let Phase::Terminal(outcome) = r.phase else {
            return Err(Error::Busy);
        };
        let endpoint = self.reference.try_clone()?;
        let r = self.request_mut(id)?;
        if r.collect_reserved {
            return Err(Error::Busy);
        }
        r.collect_reserved = true;
        Ok(Collection {
            endpoint,
            identity: id,
            client_id: r.client_id,
            outcome,
            payload: r.response.clone(),
        })
    }
    pub fn finish_collect(&mut self, collection: &Collection, copied: bool) -> Result<(), Error> {
        if collection.endpoint.id() != self.reference.id() {
            return Err(Error::Stale);
        }
        let r = self.request_mut(collection.identity)?;
        if !r.collect_reserved || !matches!(r.phase, Phase::Terminal(_)) {
            return Err(Error::Stale);
        }
        r.collect_reserved = false;
        if copied || cfg!(feature = "ipc-collect-copy-negative") {
            if cfg!(feature = "ipc-charge-release-negative") {
                let request = self.requests[collection.identity.slot]
                    .take()
                    .expect("reserved request remains present");
                let Request {
                    _consumer,
                    queue,
                    active,
                    ..
                } = request;
                core::mem::forget(_consumer);
                if let Some(charge) = queue {
                    core::mem::forget(charge);
                }
                if let Some(charge) = active {
                    core::mem::forget(charge);
                }
            }
            if !cfg!(feature = "ipc-charge-release-negative") {
                self.requests[collection.identity.slot] = None;
            }
        }
        Ok(())
    }
    pub fn abandon(&mut self, caller: ProcessId, id: RequestId) -> Result<(), Error> {
        self.consumer(caller, id)?;
        self.request_mut(id)?.alive = false;
        if let Some(wait) = self.request_mut(id)?.wait.take() {
            // This request has not published terminal wake work yet.
            assert!(!self.waits[wait].as_ref().unwrap().ready);
            self.waits[wait] = None;
        }
        if matches!(self.request(id)?.phase, Phase::Terminal(_)) {
            self.requests[id.slot] = None;
        } else if self.request(id)?.phase != Phase::Committed {
            self.terminal(id, Cause::Cancel)?;
        }
        Ok(())
    }
    pub fn process_death(&mut self, process: ProcessId) {
        if process == self.reference.service() {
            self.shutdown();
        }
        for slot in 0..R {
            if let Some(r) = &self.requests[slot]
                && r.consumer == process
            {
                let id = r.identity;
                let _ = self.abandon(process, id);
            }
        }
        // Registered waits own no remote record yet. Ready records require owner ACK,
        // including stale/dead task records, before endpoint reclamation.
        for slot in &mut self.waits {
            if slot
                .as_ref()
                .is_some_and(|s| s.wake.key.process == process && !s.ready)
            {
                *slot = None;
            }
        }
        if self
            .receive_wait
            .is_some_and(|index| self.waits[index].is_none())
        {
            self.receive_wait = None;
        }
    }
    pub fn shutdown(&mut self) {
        self.reference.close();
        for slot in 0..R {
            if let Some(r) = &self.requests[slot]
                && !matches!(r.phase, Phase::Terminal(_))
            {
                let id = r.identity;
                let _ = self.terminal(id, Cause::ServiceDeath);
            }
        }
        if let Some(wait) = self.receive_wait.take() {
            self.notify(wait);
        }
    }
    #[allow(dead_code)] // The lost-wake fault profile intentionally removes both registration call sites.
    fn reserve_wait(&mut self, key: WaitKey, target: WaitTarget) -> Result<usize, Error> {
        if self
            .waits
            .iter()
            .flatten()
            .any(|s| s.wake.key.process == key.process)
        {
            return Err(Error::Busy);
        }
        let index = self
            .waits
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Exhausted)?;
        self.waits[index] = Some(WaitSlot {
            wake: Wake { key, target },
            ready: false,
        });
        Ok(index)
    }
    /// True means condition satisfied; false means exact registration owns a wake slot.
    pub fn wait_readable(&mut self, caller: ProcessId, key: WaitKey) -> Result<bool, Error> {
        self.service(caller)?;
        if key.process != caller {
            return Err(Error::Denied);
        }
        if self.queued != 0 || self.reference.closed() {
            return Ok(true);
        }
        if self.receive_wait.is_some() {
            return Err(Error::Busy);
        }
        // Deliberately omit publication for the focused lost-wake mutation.
        // The native scheduler will still block, allowing the QEMU control to
        // prove that unregistered waiters lose progress.
        #[cfg(feature = "ipc-wait-recheck-negative")]
        {
            return Ok(false);
        }
        #[cfg(not(feature = "ipc-wait-recheck-negative"))]
        {
            let index = self.reserve_wait(key, WaitTarget::Readable(self.reference.id()))?;
            self.receive_wait = Some(index);
            // Registration-before-recheck is explicit even though endpoint exclusion
            // serializes queue mutation; no callback or scheduler borrow spans this phase.
            if self.queued != 0 || self.reference.closed() {
                self.receive_wait = None;
                self.notify(index);
                return Ok(true);
            }
            Ok(false)
        }
    }
    pub fn wait_terminal(
        &mut self,
        caller: ProcessId,
        id: RequestId,
        key: WaitKey,
    ) -> Result<bool, Error> {
        let r = self.consumer(caller, id)?;
        if key.process != caller {
            return Err(Error::Denied);
        }
        if matches!(r.phase, Phase::Terminal(_)) {
            return Ok(true);
        }
        if r.wait.is_some() {
            return Err(Error::Busy);
        }
        #[cfg(feature = "ipc-wait-recheck-negative")]
        {
            return Ok(false);
        }
        #[cfg(not(feature = "ipc-wait-recheck-negative"))]
        {
            let index = self.reserve_wait(key, WaitTarget::Terminal(self.reference.id(), id))?;
            self.request_mut(id)?.wait = Some(index);
            if matches!(self.request(id)?.phase, Phase::Terminal(_)) {
                self.request_mut(id)?.wait = None;
                self.notify(index);
                return Ok(true);
            }
            Ok(false)
        }
    }
    pub fn pending_wakes(&self) -> impl Iterator<Item = Wake> + '_ {
        self.waits
            .iter()
            .flatten()
            .filter(|s| s.ready)
            .map(|s| s.wake)
    }
    pub fn acknowledge_wake(&mut self, key: WaitKey) -> Result<(), Error> {
        let slot = self
            .waits
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.ready && s.wake.key == key))
            .ok_or(Error::Stale)?;
        self.waits[slot] = None;
        Ok(())
    }
}
impl<const Q: usize, const R: usize> Drop for Endpoint<Q, R> {
    fn drop(&mut self) {
        assert!(
            self.reclaimable(),
            "IPC endpoint release requires actual request/reference/wake quiescence"
        );
    }
}

#[cfg(test)]
mod tests;
