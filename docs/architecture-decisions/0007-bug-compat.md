# ADR-0007 — Bug compatibility and module retirement

Status: **Accepted**. Date: 2026-10-02.

## Context

An error can become a dependency, but security defects must not be preserved (cases 01, 07, 08, 23, 29).

## Decision

Fix native behavior and add regression checks. Preserve safe old semantics only in an explicit optional module backed by consumer evidence. Require owner, named consumers, creation reason, migration target, finite support window and removal condition. Explicitly name dormant/offline recovery packages, their owners and deadlines. Renew only by recorded decision with a new finite deadline; unknown hypothetical consumers cannot renew support.

Separate support expiry and default-distribution removal from runtime unloading. Expiry closes unsupported admission and preserves explicit migration errors and a registry tombstone. Bound/draining code and accepted work remain retained until runtime references, callbacks, device activity and grace periods are quiescent. Archive installation is explicit and subject to security/support limits; it never restores security-invalid behavior. See the [compatibility model](../architecture/compatibility-model.md).

This clarification addresses issue #3 through existing LAW-008; it does not add a law, production loader or automatic semantic-version support renewal.

Issue #8 makes support per semantic contract/version for both native contracts and
adapters: require introduced, deprecated, support_until, known_consumers and replacement
fields linked to owner/migration/removal records. Before deprecation, explicitly unset
fields are permitted under the stated publication/support policy; experimental versions
claim no long-term guarantee. After deprecation, a finite deadline and named replacement
or explicit end-of-service decision are mandatory. Reviewed consumer evidence and a new
bounded deadline are required for renewal. Artifact revisions and successor versions
neither renew predecessors nor authorize immediate unloading. Honor ABI-FREEZE promises;
support expiry and quiescent reclamation remain independent under LAW-008.

## Alternatives

Keep the native bug forever; break all consumers immediately; generate an adapter for every bug; remove solely on zero recent calls; ship every old module indefinitely.

## Why rejected

These undermine native correctness, prevent migration, or create unsupported maintenance debt. Zero recent calls misses declared offline obligations; perpetual default distribution defeats bounded retirement.

## Consequences

Maintain a registry, retirement records, migration windows and accounting for dormant or offline consumers.

## Compatibility impact

Reject requests incompatible with security. Old semantics do not grant a right to exploit a defect.

## Performance impact

Lifecycle accounting has costs outside the hot path; runtime metrics depend on the profile.

## Security impact

Never restore unsafe behavior or unload referenced code.

## Testing

Check migration and native-only operation. Zero observed calls cannot cancel unexpired declared support; incomplete coverage cannot renew an expired window. Host retirement-model negative controls detect freeing active references at expiry, admitting new unsupported work and losing a tombstone. Production registry/error transport, clock and runtime unload checks remain missing.

## Reversibility

A supported module may return as a new artifact before support expires; the native bug must not return.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0007-bug-compat.md)
