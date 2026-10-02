# Capability issuance and dynamic policy

## Source finding

CE combines handle rights with security-module decisions and resource-specific security contexts. Resource providers participate in authorization of named user resources. [Resource access control](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_resource_acces_control.htm). The CE 1.2 guide describes PSL-bound rules, default denial when no rule applies, stateful models and policy-test construction. [Guide, printed pages 246–249](https://support.kaspersky.com/help/KCE/1.2/en-US/KasperskyOS-CE-en-US.pdf#page=246).

This refutes a simple contrast between “KasperskyOS policies” and “KOLVRT capabilities”: the documented model composes both. A statically built policy can still make state-dependent runtime decisions. The older Education Kit also describes stateful/timed policies; do not assume its family names are the CE API. [Historical policy model, 2020](https://support.kaspersky.com/kos/educationkitbeta0.1/en-us/security_policies.htm).

## Proposed hybrid for KOLVRT

Separate native issuance from use: issuance selects object, bounded rights, caller and revocation semantics; every admission validates the handle and any live conditions necessary for correctness. Stable grants can avoid reevaluating unrelated global rules, but this is an optimization hypothesis, not a measured advantage or exemption from checks.

| Condition                    | Required live enforcement or explicit lease semantics                               |
| ---------------------------- | ----------------------------------------------------------------------------------- |
| Time/temporary delegation    | Check expiry in a monotonic epoch; do not reuse expired grants                      |
| Quotas                       | Atomically charge before publication; retain costs through terminal release         |
| Revocation/credential change | Specify linearization and accepted-work semantics; check current generation/context |
| Device reassignment          | Drain DMA/IRQs; invalidate old generation before a new owner                        |
| Network policy               | Bind permitted peer/flow; reconsider changes where the contract requires it         |
| Multi-party authority        | Retain authenticated contributions and define when consent expires                  |

## Failure modes and adoption gate

A cached “allow” followed by credential revocation is a check/use race. A valid service endpoint must not imply permission for every service-private resource. Gate new types through issue #7; global coordination alone does not require EL1. Measure issuance-only leases, targeted dynamic checks and a central policy approach on equivalent workloads before choosing. Reject a universal policy DSL now; allow a small explicit decision table until a workload proves it insufficient.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/security-policy.md)
