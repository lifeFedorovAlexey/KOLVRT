# Console, diagnostic and event output

Human prose is presentation; machine records are evidence. Use the same checked facts for both, never parse ordinary wording as a kernel ABI. This policy applies to new output across the project; the current implementation covers kernel UART and the xtask runner. Compatibility measurements retain their separate [diagnostic contract](diagnostics.md).

## Human console

Keep the project emblem, followed by KOLVRT, execution profile, architecture and exception level. Use ASCII text and the layout [LEVEL] component: message. Levels are OK for completed checks, INFO for neutral progress, WARN for degraded/incomplete observation, FAIL for failure and DEBUG for opt-in detail. Terminal color follows the palette below and never conveys meaning alone. No clearing or cursor animation; only presentation SGR colors are permitted.

Report memory with units and page size; label the observation stage. Normal boot does not dump addresses or JSON. List only completed milestones, then a concise summary. Initialization, validation, runtime readiness and shutdown are separate states. Current boot validates and powers off; it does not claim a persistent running system. CPU participation is not a post-shutdown online count. A contained test fault is not a kernel failure.

DEV permits trusted diagnostic details. PROD retains protection and failure detection but omits exception addresses and panic internals. Do not emit secrets, credentials, usable handles or user payloads. Human messages may change without versioning. New emitters use diagnostics::status; raw log! is reserved for fixed branding/header text. Kernel wording is English; maintain Russian documentation translations.

## Terminal colors

xtask automatically colors interactive stdout. --color=auto|always|never selects the command's mode. Auto disables color for redirection, nonempty NO_COLOR or TERM=dumb; explicit always takes precedence. Color the badge and immediately reset the style. Preserve the original emblem; highlight KOLVRT in green. Raw UART and machine records contain no ANSI; direct QEMU/debug UART remains plain text. Color is a host presentation layer, not a kernel security mode.

| Label           | Color       | ANSI SGR |
| --------------- | ----------- | -------- |
| OK / NATIVE     | Green       | 32       |
| FAIL / COMPAT   | Red         | 31       |
| WARN / MIXED    | Yellow      | 33       |
| MOSTLY_NATIVE   | Light green | 92       |
| LEGACY          | Brown       | 38;5;130 |
| INFO            | Cyan        | 36       |
| DEBUG / UNKNOWN | Gray        | 90       |

The state palette preserves the existing diagnostic contract. An OK color means a successful check, not NATIVE classification. The renderer styles only an explicitly published label; it never infers compatibility state from prose or zero calls. Labels, scope and evidence remain mandatory with color.

## Machine events

The machine-events build feature explicitly enables the evidence stream in either profile. Ordinary build/debug/run does not enable it. cargo xtask test enables it; cargo xtask run --machine enables it explicitly. A shared UART carries human text and newline-terminated records of the form:

```text
@KOLVRT/1 {"event":"panic","status":"fail"}
```

The prefix identifies framing version 1. One JSON object per record; no literal newline inside JSON strings, escape quotes/control bytes. Current emitters use fixed field names, trusted constant test names and numeric/boolean fields, not external string interpolation. Prefix and payload including newline are limited to 4096 bytes. Reserve @KOLVRT at line start; human prose cannot impersonate an event. No wall-clock timestamps or global multi-CPU ordering are invented.

| Event       | Required payload and interpretation                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------------------- |
| test        | name, status; unique expected test, pass or fail                                                                    |
| suite       | status, tests; terminal summary after every expected test                                                           |
| boot        | status, el, timer_irq, active_cpus, secondary_shutdown_verified; terminal boot validation, not readiness            |
| el0         | status, processes, workers, faults, switches, reclaimed; bounded exercise counts and resource outcome               |
| measurement | scope, units, frequency, warmup, iterations, median, p95, p99, samples; timer observations, not a performance score |
| fatal       | status=fail, esr; architectural syndrome; address details remain DEV-only human diagnostics                         |
| panic       | status=fail; panic internals remain DEV-only human diagnostics                                                      |

Status values are pass/fail, identifiers are English. Unknown additive fields may be retained; unknown event names, unsupported framing versions, invalid required fields, duplicate singleton/test records, missing terminal records and events after terminal success fail validation. A failure always overrides prior success. Changed mandatory semantics require a new framing version and coordinated runner changes. Historical unframed logs remain historical artifacts, not live version-1 input.

xtask saves raw UART in `target/kernel/*.log` and parsed payloads in `*.results.json`. Its console filters framed records and prints human summaries; build metadata goes to `*-build.json` with a short console line. Explicit machine mode means records are present in the raw log, not that the console becomes a JSON dump. The human run path is a presentation convenience, not an evidence validator; use test or run --machine for structured validation.

## Bounded implementation and execution contexts

The polled UART writer allocates no heap and preflights record length before output. Each byte has at most 1,000,000 TX-full register polls; timer availability is not assumed. Oversized, unavailable or timed-out writes increment a dropped-record counter. A timeout may leave a partial line; there is no queue, retry guarantee or hidden success. Missing/truncated evidence must fail host validation. If output resumes, the final human summary warns about lost records; inability to report loss remains an explicit limitation.

CPU0 alone writes, with no IRQ-handler output. CPU1 records failure through the existing SMP path; it does not become a second UART writer. No logging locks, recursion or panic on writer failure. Callers must not pass mutable/side-effectful Display implementations: length preflight and emission format twice. IRQ, device handlers and recovery paths must use bounded state publication to their owner rather than adding UART calls. The panic path masks interrupts and performs best-effort output before shutdown. Failures before UART publication may be silent; timeout/missing result detects them at the host. Removing diagnostics does not remove mandatory security or correctness (LAW-035).

## Verification and extension

Host checks reject truncated/oversized/malformed framing, wrong versions, missing/duplicate terminal results, missing/duplicate tests and failures after success. Kernel matrix checks both profiles and actual failure propagation; human runs check the absence of event records. UART preflight bounds are implementation limits, not proof of timing on silicon. Future concurrent logging, external strings or transports require a new bounded ownership/escaping argument; this policy does not introduce a logging framework.

Actual before/after provenance, DEV/PROD events and panic output are retained in the [output verification record](../../research/results/output-policy.json). The before excerpt is user-reported; after examples are actual QEMU captures.

[Russian translation](../../translations/ru/docs/architecture/output-policy.md)
