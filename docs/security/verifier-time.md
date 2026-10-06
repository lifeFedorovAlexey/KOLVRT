# Provisioned verifier time and session lifetime

Document status: CURRENT
Evidence scope: bounded host-side correction for #71; injected-clock regression evidence, not production clock authenticity, live revocation or crash-durable replay acceptance.
Current reference: [Provenance implementation](../../crates/migration-advisor/src/provenance.rs)

<a name="kolvrt-verifier-time"></a>

## Temporal boundary

`SignedArtifactStore::provisioned` takes an application-configured `Arc<dyn TrustedUtcClock>`, not a constructor-supplied integer timestamp. The clock and policy are never deserialized from the request. The CLI uses `SystemUtcClock`; trusting OS UTC is an explicit host assumption, not a production authenticated-time service.

Construction establishes an initial observation. Session admission checks current time before validating the signed challenge and again after ledger I/O, immediately before publishing the active binding. Each provisioned `authenticate` checks at entry and before returning success. Both checks validate session expiry/future skew, issuer eligibility and evidence-key validity. A session expiring at time T is rejected at T. The second pass filters already verified immutable attestations and rechecks producer-independence thresholds; it repeats neither signature cryptography nor artifact hashing.

An accepted result is valid as of its final observation. It is not a lease for later deployment: deployments still require independent live admission and authorization. Stable artifacts remain the store owner's responsibility.

## Clock and concurrency failure

Unavailable time, mutex poisoning or an observation earlier than this verifier's last accepted time fails closed. Unavailability or rollback permanently invalidates that instance; later recovery does not revive it. A new externally provisioned verifier and fresh issuer challenge are required. Concurrent observations arriving out of temporal order are conservatively rejected too; integrations needing concurrent use must account for that availability limitation.

Clock callbacks execute outside verifier mutexes. The provider is trusted to supply progressing, authentic UTC or report failure. Checking monotonic observations cannot detect a malicious frozen clock or prove clock authenticity. `SystemTime` is explicitly [non-monotonic in Rust](https://doc.rust-lang.org/std/time/struct.SystemTime.html). [RFC 7519 expiry semantics](https://www.rfc-editor.org/rfc/rfc7519#section-4.1.4) provide a related primary example of comparing expiry with current time, not construction time; this custom signature protocol is not JWT and does not adopt JWT authority.

Each instance admits at most one provisioned session after challenge validation. A compare-and-swap-equivalent atomic latch precedes replay-marker creation; another session cannot overwrite the active binding or mix evidence from different sessions during one authentication. Failed validation before that admission point may retry; once ledger admission starts, failure consumes the instance. A failed final clock/key check leaves the challenge marker in place and does not activate a session. Markers are never deleted to make a retry succeed.

## Key policy and remaining gates

Keys, revoked/compromised flags and producer domains are an immutable policy snapshot. Valid-from/valid-until windows, including the session issuer's, are checked against current time throughout the active session. Applying a new revocation/rotation policy requires retiring old verifier instances and creating new externally configured ones; online revocation, forced retirement enforcement and secure policy distribution remain production work. This change does not claim that an old instance observes an external policy edit.

Replay remains create-once host filesystem admission. File/namespace durable commit, supported storage/crash model, corruption recovery, pruning and process-kill/power-loss tests remain open in #71; #14 production admission is not complete. This correction adds no durability claim, install operation, routing mutation, kernel dependency or runtime IPC mechanism.

The Rust constructor API intentionally changes: callers must provide a clock implementation rather than preserve the unsafe snapshot lifetime. Experimental schema 1 remains separate. Existing implementation order does not justify retaining the old interface.

## Verification scope

[Provisioned tests](../../crates/migration-advisor/tests/provisioned_provenance.rs) use private synthetic clocks and Ed25519 keys. They cover constructor/admission/authentication expiry, future sessions, unavailable/rollback clocks and sticky invalidation, issuer/producer-key validity changes, final-check expiry/failure, consumed-but-unpublished challenges and concurrent distinct-session admission. [Migration tests](../../crates/migration-advisor/tests/migration.rs) retain the signed planning/authorization chain and CLI checks with the new interface.

[Exact-source receipt](../../research/results/issue71-verifier-time.json) records only host verification of these declared bytes. It is not authenticated time, real custody independence, physical ARM64 or storage power-loss evidence. Full #71 closure requires the remaining gates above.

[Russian translation](../../translations/ru/docs/security/verifier-time.md)

<!-- knowledge -->

```json
{
  "kind": "subsystem-contract",
  "summary": "Current-clock host verifier acceptance with sticky failure and immutable session/policy boundaries.",
  "units": [
    {
      "summary": "Current-clock host verifier acceptance with sticky failure and immutable session/policy boundaries.",
      "depends_on": [
        "law.009",
        "law.013",
        "law.036",
        "law.040",
        "adr.0013",
        "adr.0008"
      ],
      "tags": ["trust", "clock", "freshness", "session", "migration"],
      "kind": "feature",
      "id": "kolvrt.security.verifier-time",
      "feature": {
        "verification": [
          {
            "scope": "Host verification mechanism with injected fixture clocks; no production authenticated time, live revocation or storage crash guarantees.",
            "receipt_sha256": "fd273b9171688235813dddb0b7e83492b6a63dbd2f405676588bbe7836c68e67",
            "environment": "host-process",
            "receipt": "research/results/issue71-verifier-time.json",
            "reason": "Phase 3.7 immutable ELF grants/client SEND binding or workspace inputs changed; matching-source verification must be refreshed. Historical receipts remain immutable.",
            "state": "STALE"
          },
          {
            "state": "NOT_APPLICABLE",
            "environment": "physical-arm64",
            "reason": "Host verifier mechanics do not establish physical KOLVRT execution or hardware clock trust."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "acceptance": ["research/results/issue71-verifier-time.json"],
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First enrolled corrected temporal verifier scope with exact-source host evidence; durable replay and production trust gates remain open.",
            "from": "UNRECORDED"
          }
        ],
        "implementation_scope": "Current-time session/key admission and final-acceptance revalidation, sticky clock failure/rollback rejection and single active provisioned session per host verifier.",
        "next_gate": "Complete #71 durable replay, recovery and live policy/time integration without upgrading synthetic evidence into production trust.",
        "roadmap_gate": "Migration trust #71",
        "issues": [71, 14],
        "sources": [
          "crates/migration-advisor/src/provenance.rs",
          "crates/migration-advisor/src/main.rs",
          "crates/migration-advisor/tests/provisioned_provenance.rs",
          "crates/migration-advisor/tests/migration.rs",
          "crates/migration-advisor/Cargo.toml",
          "Cargo.lock",
          "rust-toolchain.toml"
        ],
        "adrs": ["adr.0013", "adr.0008"],
        "limitations": [
          "Immutable policy snapshot does not observe online revocation. Trusted provider authenticity/progress, storage-crash durability, recovery/pruning and production admission remain open; quantum/kernel execution is not involved."
        ],
        "acceptance": ["research/results/issue71-verifier-time.json"],
        "implementation": "BOUNDED_IMPLEMENTED"
      },
      "anchor": "kolvrt-verifier-time"
    }
  ],
  "id": "doc.kolvrt.security.verifier-time",
  "schema_version": 1
}
```
