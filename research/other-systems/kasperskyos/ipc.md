# IPC security

## Source finding

CE 1.2 checks request and response structure, evaluates applicable policy, and lets the kernel deliver or reject the message. [IPC control](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_ipc_control.htm).

seL4 documents endpoint IPC, capability transfer and a conditional fastpath; its fastpath does not admit capability transfer. [IPC tutorial](https://docs.sel4.systems/Tutorials/ipc.html).

## KOLVRT recommendation

Keep copied bounded request snapshots, caller-context identity, handle generation/type checks, native rights and charge retention at admission. Separate valid encoding, authorized effects, queue acceptance and terminal outcome. Typed messages do not prove authorization. A response is adversarial data: verify ID, length, status and duplicate completion.

Do not choose synchronous versus asynchronous transport from a label. Synchronous nested service chains can deadlock; asynchronous queues need quotas, cancellation and retained owners. Shared buffers need a snapshot or immutable lease, permissions, ordering and a completion contract. An IPC badge is evidence of a transported grant, not arbitrary application-user identity.

## Validation gate

The existing host decoder and lifetime models are evidence only for their represented cases. Later combined kernel tests must inject malformed bytes, foreign/stale handles, identity spoofing, response replay, owner death and post-commit cancellation. Denial occurs before unauthorized effects; an unknown effect never justifies replay. Compare transport options with identical authority/outcome guarantees. No IPC latency or general-purpose scalability result is established here.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/ipc.md)
