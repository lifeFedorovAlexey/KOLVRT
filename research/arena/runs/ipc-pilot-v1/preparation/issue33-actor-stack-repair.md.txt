# IPC actor DEV stack repair (functional preparation)

Retained failed execution: target/kernel/arena-ipc/1791411495355-39236, case0 DEV/OFF; root data abort class36, empty report, resources retained. Kernel events contain no FAR/PC, so exact faulting instruction is not independently available from that run.

A concrete actor stack violation was established in emitted DEV AArch64 assembly before edits: native_main7376 + finish2960 + rpc_words656 + SDKrpc4384 + SDKcall1712 + frame1008 =18096 bytes, exceeding USER_STACK_PAGES4 *4096=16384 before smaller callees. The path queries the server ledger after sample completion, consistent with the retained execution reaching37 server receive blocks and no final report. No kernel timing/scheduling defect is inferred.

Fix changes external actor lifetime structure only: small native_main owns records; setup, measure, finish separate; saturation B rejection has no retained Collect buffer, and C recovery sample executes after saturation A frame has returned. Existing real IPC flow, inherited one-second deadline, three CLOCK calls/attempt, observer modes and report schema remain.

Current emitted principal chains:
- root finish ledger:3136+2960+656+4384+1712+1008=13856 bytes;
- saturation B reject:3136+3936+3200+1072+1712+1008=14064 bytes;
- peer export:6352+656+4384+1712+1008=14112 bytes.
These totals cover named principal frames, not a universal whole-program stack bound or successful guest execution. Smaller helpers still consume stack. SDK and kernel stack capacity are unchanged. Emitted files are target/aarch64-unknown-none/debug/deps/{selftest_ipc_measure_root,selftest_ipc_measure_peer,selftest_ipc_measure_client,native_userspace}-*.s.

Actual verification after repair: AArch64 DEV assembly builds for all3 actors PASS; PROD crossbuild PASS; AArch64 Clippy PASS; importing actor-protocol host tests3/3PASS. Parent to execute new-source functional smoke; prior failed evidence preserved. No rerun was made before diagnosing the source defect.
