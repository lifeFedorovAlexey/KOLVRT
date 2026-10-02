//! Finite synthetic binding policy; not a trusted policy loader or runtime router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Native,
    NativeReplica,
    Compat,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    BeforeAdmission,
    Committed,
    EffectUnknown,
}
#[derive(Clone, Copy)]
pub struct Policy {
    pub alternate: Route,
    pub production_authorized: bool,
}
#[derive(Clone, Copy)]
pub struct Preconditions {
    pub production: bool,
    pub failure: Failure,
    pub authority_preserved: bool,
    pub quotas_preserved: bool,
    pub state_compatible: bool,
    pub domain_drained: bool,
    pub consumer_opt_in: bool,
    pub new_binding_generation: u64,
}
/// Mandatory binding result, independent of optional diagnostics.
#[derive(Debug, PartialEq, Eq)]
pub struct Transition {
    pub from: Route,
    pub to: Route,
    pub generation: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Denial {
    Policy,
    Replay,
    Authority,
    Quota,
    State,
    Semantics,
    Generation,
}
/// Assertions here are fixture inputs, not authenticated production evidence.
/// Acceptance creates a new binding result; no request is executed or replayed.
pub fn bind_alternate(
    policy: Option<Policy>,
    old_generation: u64,
    p: Preconditions,
) -> Result<Transition, Denial> {
    let policy = policy.ok_or(Denial::Policy)?;
    if (p.production && !policy.production_authorized) || policy.alternate == Route::Native {
        return Err(Denial::Policy);
    }
    if p.failure != Failure::BeforeAdmission {
        return Err(Denial::Replay);
    }
    if !p.authority_preserved {
        return Err(Denial::Authority);
    }
    if !p.quotas_preserved {
        return Err(Denial::Quota);
    }
    if !p.state_compatible || !p.domain_drained {
        return Err(Denial::State);
    }
    if policy.alternate == Route::Compat && !p.consumer_opt_in {
        return Err(Denial::Semantics);
    }
    if p.new_binding_generation <= old_generation {
        return Err(Denial::Generation);
    }
    Ok(Transition {
        from: Route::Native,
        to: policy.alternate,
        generation: p.new_binding_generation,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Preconditions {
        Preconditions {
            production: true,
            failure: Failure::BeforeAdmission,
            authority_preserved: true,
            quotas_preserved: true,
            state_compatible: true,
            domain_drained: true,
            consumer_opt_in: false,
            new_binding_generation: 2,
        }
    }
    fn replica() -> Option<Policy> {
        Some(Policy {
            alternate: Route::NativeReplica,
            production_authorized: true,
        })
    }
    #[test]
    fn defaults_and_production_need_explicit_policy() {
        assert_eq!(bind_alternate(None, 1, ready()), Err(Denial::Policy));
        let policy = Some(Policy {
            alternate: Route::NativeReplica,
            production_authorized: false,
        });
        assert_eq!(bind_alternate(policy, 1, ready()), Err(Denial::Policy));
        assert!(
            bind_alternate(
                policy,
                1,
                Preconditions {
                    production: false,
                    ..ready()
                }
            )
            .is_ok()
        );
    }
    #[test]
    fn accepted_transition_is_a_mandatory_result() {
        assert_eq!(
            bind_alternate(replica(), 1, ready()),
            Ok(Transition {
                from: Route::Native,
                to: Route::NativeReplica,
                generation: 2
            })
        );
        let policy = Some(Policy {
            alternate: Route::Compat,
            production_authorized: true,
        });
        assert_eq!(bind_alternate(policy, 1, ready()), Err(Denial::Semantics));
        let result = bind_alternate(
            policy,
            1,
            Preconditions {
                consumer_opt_in: true,
                ..ready()
            },
        )
        .unwrap();
        assert_eq!(result.to, Route::Compat);
        assert_eq!(result.generation, 2);
    }
    #[test]
    fn authority_quota_state_and_generation_cannot_be_downgraded() {
        for (p, denial) in [
            (
                Preconditions {
                    authority_preserved: false,
                    ..ready()
                },
                Denial::Authority,
            ),
            (
                Preconditions {
                    quotas_preserved: false,
                    ..ready()
                },
                Denial::Quota,
            ),
            (
                Preconditions {
                    state_compatible: false,
                    ..ready()
                },
                Denial::State,
            ),
            (
                Preconditions {
                    domain_drained: false,
                    ..ready()
                },
                Denial::State,
            ),
            (
                Preconditions {
                    new_binding_generation: 1,
                    ..ready()
                },
                Denial::Generation,
            ),
        ] {
            assert_eq!(bind_alternate(replica(), 1, p), Err(denial));
        }
    }
    #[test]
    fn committed_or_unknown_effects_never_authorize_replay() {
        for failure in [Failure::Committed, Failure::EffectUnknown] {
            assert_eq!(
                bind_alternate(replica(), 1, Preconditions { failure, ..ready() }),
                Err(Denial::Replay)
            );
        }
    }
}
