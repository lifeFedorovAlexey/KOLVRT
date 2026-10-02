//! Finite transition systems; exhaustive over their stated domains, not a kernel proof.
#![forbid(unsafe_code)]
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
};

pub struct Exploration {
    pub states: usize,
    pub transitions: usize,
    pub counterexample: Option<Vec<String>>,
}
pub fn explore<S: Clone + Ord + Debug>(
    initial: S,
    successors: impl Fn(&S) -> Vec<(&'static str, S)>,
    invariant: impl Fn(&S) -> bool,
) -> Exploration {
    let mut seen = BTreeMap::from([(initial.clone(), None::<(S, &'static str)>)]);
    let mut queue = VecDeque::from([initial]);
    let mut transitions = 0;
    while let Some(state) = queue.pop_front() {
        if !invariant(&state) {
            let mut trace = vec![format!("violation: {state:?}")];
            let mut cursor = &state;
            while let Some((parent, action)) = seen[cursor].as_ref() {
                trace.push((*action).into());
                cursor = parent;
            }
            trace.reverse();
            return Exploration {
                states: seen.len(),
                transitions,
                counterexample: Some(trace),
            };
        }
        for (action, next) in successors(&state) {
            transitions += 1;
            if !seen.contains_key(&next) {
                seen.insert(next.clone(), Some((state.clone(), action)));
                queue.push_back(next);
            }
        }
    }
    Exploration {
        states: seen.len(),
        transitions,
        counterexample: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pending {
    Unsent,
    Queued,
    Effect,
    Done,
    Cancelled,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lifetime {
    pub handles: [bool; 2],
    pub requests: [Pending; 2],
    pub charged: u8,
    pub revoked: bool,
    pub service_alive: bool,
    pub freed: bool,
    pub effects: [u8; 2],
}
impl Default for Lifetime {
    fn default() -> Self {
        Self {
            handles: [true; 2],
            requests: [Pending::Unsent; 2],
            charged: 0,
            revoked: false,
            service_alive: true,
            freed: false,
            effects: [0; 2],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    None,
    FreeOnClose,
    DoubleEffect,
    LoseCharge,
}
fn active(p: Pending) -> bool {
    matches!(p, Pending::Queued | Pending::Effect)
}
impl Lifetime {
    pub fn invariant(&self) -> bool {
        let retained = self.requests.iter().filter(|&&p| active(p)).count();
        self.charged as usize == retained
            && self.charged <= 1
            && !(self.freed && (self.handles.contains(&true) || retained > 0))
            && self.effects.iter().all(|&n| n <= 1)
            && self
                .requests
                .iter()
                .zip(self.effects)
                .all(|(&p, e)| p != Pending::Cancelled || e == 0)
    }
    pub fn steps(&self, mutation: Mutation) -> Vec<(&'static str, Self)> {
        let mut out = Vec::new();
        for i in 0..2 {
            if self.handles[i] {
                let mut n = self.clone();
                n.handles[i] = false;
                out.push((if i == 0 { "close A" } else { "close B" }, n));
            }
            if self.requests[i] == Pending::Unsent
                && self.handles[i]
                && !self.revoked
                && self.service_alive
                && !self.freed
                && self.charged < 1
            {
                let mut n = self.clone();
                n.requests[i] = Pending::Queued;
                n.charged += 1;
                out.push((if i == 0 { "admit A" } else { "admit B" }, n));
            }
            if self.requests[i] == Pending::Queued {
                let mut n = self.clone();
                n.requests[i] = Pending::Cancelled;
                n.charged -= 1;
                out.push(("cancel before effect", n));
                if self.service_alive {
                    let mut n = self.clone();
                    n.requests[i] = Pending::Effect;
                    n.effects[i] += 1;
                    if mutation == Mutation::LoseCharge {
                        n.charged -= 1;
                    }
                    out.push(("commit effect", n));
                }
            }
            if self.requests[i] == Pending::Effect {
                let mut n = self.clone();
                n.requests[i] = Pending::Done;
                n.charged = n.charged.saturating_sub(1);
                out.push(("complete after effect (cancel cannot roll back)", n));
                if mutation == Mutation::DoubleEffect && self.effects[i] < 2 {
                    let mut n = self.clone();
                    n.effects[i] += 1;
                    out.push(("BUG retry committed effect", n));
                }
            }
            if active(self.requests[i]) && !self.service_alive {
                let mut n = self.clone();
                n.requests[i] = Pending::Failed;
                n.charged = n.charged.saturating_sub(1);
                out.push(("observe service failure; effect may be unknown", n));
            }
        }
        if !self.revoked {
            let mut n = self.clone();
            n.revoked = true;
            out.push(("revoke future admission", n));
        }
        if self.service_alive {
            let mut n = self.clone();
            n.service_alive = false;
            out.push(("service crash", n));
        }
        if !self.freed
            && !self.handles.contains(&true)
            && (self.charged == 0 || mutation == Mutation::FreeOnClose)
        {
            let mut n = self.clone();
            n.freed = true;
            out.push(("reclaim storage", n));
        }
        out
    }
}
pub fn lifetime(mutation: Mutation) -> Exploration {
    explore(
        Lifetime::default(),
        |s| s.steps(mutation),
        Lifetime::invariant,
    )
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wait {
    phase: u8,
    ready: bool,
    notified: bool,
    deadline: bool,
    terminal: u8,
}
pub fn wait(broken: bool) -> Exploration {
    explore(
        Wait {
            phase: 0,
            ready: false,
            notified: false,
            deadline: false,
            terminal: 0,
        },
        |s| {
            let mut out = Vec::new();
            if !s.ready {
                let mut n = s.clone();
                n.ready = true;
                if n.phase == 2 {
                    n.notified = true;
                }
                out.push(("publish event", n));
            }
            if s.phase == 0 {
                let mut n = s.clone();
                if s.ready {
                    n.phase = 3;
                    n.terminal = 1;
                } else {
                    n.phase = 1;
                }
                out.push(("initial probe", n));
            }
            if s.phase == 1 {
                let mut n = s.clone();
                n.phase = 2;
                if !broken && n.ready {
                    n.notified = true;
                }
                out.push(("register and recheck", n));
            }
            if s.phase == 2 && s.terminal == 0 && s.notified {
                let mut n = s.clone();
                n.terminal = 1;
                n.phase = 3;
                out.push(("wake wins", n));
            }
            if !s.deadline {
                let mut n = s.clone();
                n.deadline = true;
                out.push(("deadline reached", n));
            }
            if s.phase == 2 && s.terminal == 0 && s.deadline {
                let mut n = s.clone();
                n.terminal = 2;
                n.phase = 3;
                out.push(("timeout wins", n));
            }
            if s.phase == 2 && s.terminal == 0 {
                let mut n = s.clone();
                n.terminal = 3;
                n.phase = 3;
                out.push(("cancel wins", n));
            }
            out
        },
        |s| !(s.phase == 2 && s.ready && !s.notified),
    )
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    admission: bool,
    in_flight: u8,
    generation: u8,
    old_released: bool,
}
pub fn binding(broken: bool) -> Exploration {
    explore(
        Binding {
            admission: true,
            in_flight: 0,
            generation: 0,
            old_released: false,
        },
        |s| {
            let mut out = Vec::new();
            if s.admission && s.generation == 0 && s.in_flight < 2 {
                let mut n = s.clone();
                n.in_flight += 1;
                out.push(("admit old generation", n));
            }
            if s.admission && s.generation == 0 {
                let mut n = s.clone();
                n.admission = false;
                out.push(("stop entire domain", n));
            }
            if s.in_flight > 0 {
                let mut n = s.clone();
                n.in_flight -= 1;
                out.push(("drain old request", n));
            }
            if !s.admission && s.generation == 0 && (s.in_flight == 0 || broken) {
                let mut n = s.clone();
                n.generation = 1;
                n.old_released = true;
                n.admission = true;
                out.push(("commit new binding and retire old code", n));
            }
            out
        },
        |s| !s.old_released || s.in_flight == 0,
    )
}

/// A bounded progress model: a ready task receives a turn within N dispatches.
/// It assumes timer delivery and a bounded non-preemptible kernel section.
pub fn round_robin(ready: [bool; 3], cursor: usize) -> Option<usize> {
    (1..=3).map(|n| (cursor + n) % 3).find(|&i| ready[i])
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Startup {
    phase: u8,
    in_flight: u8,
}
pub fn startup(broken: bool) -> Exploration {
    explore(
        Startup {
            phase: 0,
            in_flight: 0,
        },
        |s| {
            let mut out = Vec::new();
            if s.phase < 3 {
                let mut n = s.clone();
                n.phase += 1;
                out.push(("initialize next dependency before publication", n));
            }
            if (1..3).contains(&s.phase) {
                let mut n = s.clone();
                n.phase = 6;
                out.push(("unpublished initialization fails; unwind dependencies", n));
            }
            if s.phase == 3 && s.in_flight < 2 {
                let mut n = s.clone();
                n.in_flight += 1;
                out.push(("admit request", n));
            }
            if s.phase == 3 {
                let mut n = s.clone();
                n.phase = 4;
                out.push(("stop admission", n));
            }
            if s.in_flight > 0 && s.phase < 5 {
                let mut n = s.clone();
                n.in_flight -= 1;
                out.push(("finish accepted request", n));
            }
            if s.phase == 4 && (s.in_flight == 0 || broken) {
                let mut n = s.clone();
                n.phase = 5;
                out.push(("quiesce interrupt and callback sources", n));
            }
            if s.phase == 5 {
                let mut n = s.clone();
                n.phase = 6;
                out.push(("release mappings", n));
            }
            out
        },
        |s| s.phase < 5 || s.in_flight == 0,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grant {
    pub object: u64,
    pub binding: u64,
    pub rights: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Domain {
    slots: [Option<Grant>; 2],
    generations: [u32; 2],
}
impl Domain {
    pub fn insert(&mut self, grant: Grant) -> Result<u64, native_protocol_model::Error> {
        let slot = self
            .slots
            .iter()
            .enumerate()
            .position(|(i, slot)| slot.is_none() && self.generations[i] < u32::MAX)
            .ok_or(native_protocol_model::Error::Exhausted)?;
        let generation = native_protocol_model::next_generation(self.generations[slot])?;
        self.slots[slot] = Some(grant);
        self.generations[slot] = generation;
        Ok(native_protocol_model::handle(slot as u32, generation))
    }
    pub fn get(&self, token: u64) -> Result<Grant, native_protocol_model::Error> {
        let slot = (token as u32) as usize;
        let generation = *self
            .generations
            .get(slot)
            .ok_or(native_protocol_model::Error::Stale)?;
        native_protocol_model::resolve(token, slot as u32, generation, self.slots[slot].is_some())?;
        self.slots[slot].ok_or(native_protocol_model::Error::Stale)
    }
    pub fn close(&mut self, token: u64) -> Result<(), native_protocol_model::Error> {
        self.get(token)?;
        self.slots[token as u32 as usize] = None;
        Ok(())
    }
    pub fn transfer(
        &self,
        token: u64,
        receiver: &mut Self,
        rights: u32,
    ) -> Result<u64, native_protocol_model::Error> {
        let mut grant = self.get(token)?;
        grant.rights = native_protocol_model::delegate(grant.rights, rights)?;
        receiver.insert(grant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_lifetime_interleavings() {
        assert!(lifetime(Mutation::None).counterexample.is_none());
    }
    #[test]
    fn lifetime_negative_controls() {
        for m in [
            Mutation::FreeOnClose,
            Mutation::DoubleEffect,
            Mutation::LoseCharge,
        ] {
            assert!(lifetime(m).counterexample.is_some(), "{m:?}");
        }
    }
    #[test]
    fn registration_has_no_lost_wakeup() {
        assert!(wait(false).counterexample.is_none());
        assert!(wait(true).counterexample.is_some());
    }
    #[test]
    fn migration_drains_before_retirement() {
        assert!(binding(false).counterexample.is_none());
        assert!(binding(true).counterexample.is_some());
    }
    #[test]
    fn every_ready_set_has_bounded_service() {
        for mask in 1..8 {
            let ready = core::array::from_fn(|i| mask & (1 << i) != 0);
            for start in 0..3 {
                let mut cursor = start;
                let mut served = [false; 3];
                for _ in 0..3 {
                    cursor = round_robin(ready, cursor).unwrap();
                    served[cursor] = true;
                }
                for i in 0..3 {
                    assert!(!ready[i] || served[i]);
                }
            }
        }
        assert_eq!(round_robin([false; 3], 0), None);
    }
    #[test]
    fn startup_and_shutdown_preserve_dependencies() {
        assert!(startup(false).counterexample.is_none());
        assert!(startup(true).counterexample.is_some());
    }
    #[test]
    fn transfer_preserves_binding_and_attenuates_authority() {
        let mut a = Domain::default();
        let mut b = Domain::default();
        let token = a
            .insert(Grant {
                object: 42,
                binding: 7,
                rights: 3,
            })
            .unwrap();
        assert!(b.get(token).is_err());
        let received = a.transfer(token, &mut b, 1).unwrap();
        assert_eq!(
            b.get(received).unwrap(),
            Grant {
                object: 42,
                binding: 7,
                rights: 1
            }
        );
        assert!(b.transfer(received, &mut a, 3).is_err());
        a.close(token).unwrap();
        assert!(a.get(token).is_err());
        assert!(a.close(token).is_err());
        assert_eq!(b.get(received).unwrap().object, 42);
        let replacement = a
            .insert(Grant {
                object: 99,
                binding: 8,
                rights: 1,
            })
            .unwrap();
        assert_ne!(replacement, token);
        assert!(a.get(token).is_err());
    }
    #[test]
    fn failed_transfer_is_transactional() {
        let mut a = Domain::default();
        let mut b = Domain::default();
        let grant = Grant {
            object: 1,
            binding: 0,
            rights: 3,
        };
        let token = a.insert(grant).unwrap();
        b.insert(grant).unwrap();
        b.insert(grant).unwrap();
        let before = b.clone();
        assert!(a.transfer(token, &mut b, 1).is_err());
        assert_eq!(b, before);
        assert_eq!(a.get(token).unwrap(), grant);
    }
    #[test]
    fn exhausted_generation_retires_only_its_slot() {
        let mut domain = Domain::default();
        domain.generations[0] = u32::MAX;
        let grant = Grant {
            object: 1,
            binding: 0,
            rights: 1,
        };
        let token = domain.insert(grant).unwrap();
        assert_eq!(token as u32, 1);
        assert_eq!(domain.get(token).unwrap(), grant);
        assert_eq!(domain.generations[0], u32::MAX);
        assert_eq!(
            domain.insert(grant),
            Err(native_protocol_model::Error::Exhausted)
        );
    }
}
