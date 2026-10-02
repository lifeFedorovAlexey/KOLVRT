//! Finite support-expiry model. No clock, loader or package registry is implemented.
use crate::{Exploration, explore};

const MAX_REFERENCES: u8 = 2;
const MAX_REQUESTS: u8 = 2;

#[derive(Clone, Copy)]
pub enum Mutation {
    None,
    FreeAtDeadline,
    AdmitAfterDeadline,
    ForgetTombstone,
    RenewOnSuccessor,
    FreeOnSuccessor,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Module {
    successor_created: bool,
    deadline_was_reached: bool,
    deadline_reached: bool,
    references: u8,
    requests: u8,
    removed: bool,
    tombstone: bool,
    admitted_after_deadline: bool,
}

pub fn retirement(mutation: Mutation) -> Exploration {
    explore(
        Module {
            successor_created: false,
            deadline_was_reached: false,
            deadline_reached: false,
            references: 0,
            requests: 0,
            removed: false,
            tombstone: false,
            admitted_after_deadline: false,
        },
        |s| {
            let mut out = Vec::new();
            if !s.successor_created {
                let mut n = s.clone();
                n.successor_created = true;
                if matches!(mutation, Mutation::RenewOnSuccessor) {
                    n.deadline_reached = false;
                }
                if matches!(mutation, Mutation::FreeOnSuccessor) {
                    n.removed = true;
                }
                out.push(("introduce independent successor version", n));
            }
            if !s.deadline_reached {
                let mut n = s.clone();
                n.deadline_reached = true;
                n.deadline_was_reached = true;
                n.tombstone = !matches!(mutation, Mutation::ForgetTombstone);
                out.push(("support deadline reached; close admission", n));
            }
            if !s.removed
                && (!s.deadline_reached || matches!(mutation, Mutation::AdmitAfterDeadline))
            {
                if s.references < MAX_REFERENCES {
                    let mut n = s.clone();
                    n.references += 1;
                    n.admitted_after_deadline |= s.deadline_reached;
                    out.push(("bind module", n));
                }
                if s.references > 0 && s.requests < MAX_REQUESTS {
                    let mut n = s.clone();
                    n.requests += 1;
                    n.admitted_after_deadline |= s.deadline_reached;
                    out.push(("admit request", n));
                }
            }
            if s.references > 0 {
                let mut n = s.clone();
                n.references -= 1;
                out.push(("release binding reference", n));
            }
            if s.requests > 0 {
                let mut n = s.clone();
                n.requests -= 1;
                out.push(("complete retained request", n));
            }
            if s.deadline_reached
                && !s.removed
                && ((s.references == 0 && s.requests == 0)
                    || matches!(mutation, Mutation::FreeAtDeadline))
            {
                let mut n = s.clone();
                n.removed = true;
                out.push(("remove unreferenced implementation", n));
            }
            out
        },
        |s| {
            (!s.removed || (s.deadline_reached && s.references == 0 && s.requests == 0))
                && (!s.deadline_was_reached || s.deadline_reached)
                && (!s.deadline_reached || s.tombstone)
                && !s.admitted_after_deadline
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_preserves_bindings_requests_and_tombstone() {
        let result = retirement(Mutation::None);
        assert!(result.counterexample.is_none());
    }

    #[test]
    fn deadline_cannot_authorize_free_or_new_admission() {
        for mutation in [Mutation::FreeAtDeadline, Mutation::AdmitAfterDeadline] {
            assert!(retirement(mutation).counterexample.is_some());
        }
    }

    #[test]
    fn unsupported_lookup_requires_retained_tombstone() {
        assert!(
            retirement(Mutation::ForgetTombstone)
                .counterexample
                .is_some()
        );
    }

    #[test]
    fn successor_neither_renews_support_nor_authorizes_removal() {
        assert!(retirement(Mutation::None).counterexample.is_none());
        for mutation in [Mutation::RenewOnSuccessor, Mutation::FreeOnSuccessor] {
            assert!(retirement(mutation).counterexample.is_some());
        }
    }
}
