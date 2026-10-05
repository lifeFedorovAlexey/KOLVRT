.section .rodata.ipc_image,"a"
.balign 16
.global ipc_image_start, ipc_image_end
.macro ipc_header op, selector=x21
 mov x0,#1
 strh w0,[x19]
 mov x0,#\op
 strh w0,[x19,#2]
 mov x0,#40
 str w0,[x19,#4]
 str \selector,[x19,#8]
 stp xzr,xzr,[x19,#16]
 str xzr,[x19,#32]
.endm
.macro ipc_call length=40
 mov x1,#\length
 add x2,x19,#512
 mov x3,#296
 bl .Lipc_do_call
 cbnz x0,.Lipc_fail
.endm
.macro ipc_bad_copy
 mov x1,#40
 mov x2,#0
 mov x3,#296
 bl .Lipc_do_call
 cmp x0,#8
 b.ne .Lipc_fail
.endm
ipc_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x28,#100
 cmp x20,#16
 b.eq .Lipc_cancel_client
 cmp x20,#18
 b.eq .Lipc_cancel_client
 cmp x20,#17
 b.eq .Lipc_cancel_service
 cmp x20,#19
 b.eq .Lipc_cancel_service
 cmp x20,#2
 b.eq .Lipc_deadline_client
 cmp x20,#4
 b.eq .Lipc_deadline_client
 cmp x20,#3
 b.eq .Lipc_deadline_service
 cmp x20,#5
 b.eq .Lipc_deadline_service
 cmp x20,#6
 b.eq .Lipc_deadline_client
 cmp x20,#8
 b.eq .Lipc_deadline_client
 cmp x20,#10
 b.eq .Lipc_deadline_client
 cmp x20,#7
 b.eq .Lipc_queued_service_death
 cmp x20,#9
 b.eq .Lipc_delivered_service_death
 cmp x20,#11
 b.eq .Lipc_committed_service_death
 cmp x20,#13
 b.eq .Lipc_receive_copy_recovery
 cmp x20,#14
 b.eq .Lipc_basic_client
 cbnz x20,.Lipc_service
.Lipc_basic_client:
 // Delay only this finite computation so the service first blocks on receive.
 mov x8,#0x20
 lsl x8,x8,#16
.Lipc_client_delay:
 subs x8,x8,#1
 b.ne .Lipc_client_delay
 ipc_header 1
 mov x0,#42
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0x1234
 str x0,[x19,#40]
 mov x28,#101
 ipc_call 48
 mov x22,x1
 cbz x22,.Lipc_fail
 // Source bytes change after submit. Service must receive the copied snapshot.
 mov x0,#0xabcd
 str x0,[x19,#40]
 // Close SEND, then operate solely through the accepted requester receipt.
 mov x0,#1
 str x0,[x19]
 mov x0,#2
 str x0,[x19,#8]
 str x21,[x19,#16]
 mov x0,#3
 str x0,[x19,#24]
 stp xzr,xzr,[x19,#32]
 mov x0,x19
 svc #0x80
 mov x28,#102
 cbnz x0,.Lipc_fail
 ipc_header 5,x22
 mov x28,#103
 ipc_call
 cmp x20,#14
 b.ne .Lipc_collect_ready
 ipc_header 6,x22
 mov x28,#122
 ipc_bad_copy
.Lipc_collect_ready:
 ipc_header 6,x22
 mov x28,#104
 ipc_call
 cmp x1,#32
 b.ne .Lipc_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lipc_fail
 ldr x0,[x9,#8]
 cmp x0,#42
 b.ne .Lipc_fail
 ldr x0,[x9,#24]
 mov x1,#0x5678
 cmp x0,x1
 b.ne .Lipc_fail
 b .Lipc_pass
.Lipc_service:
 cbz x23,.Lipc_current_service_receive
 // Bootstrap passes only a numeric old token. A freshly created service and
 // endpoint must reject it through their actual new receiver binding.
 ipc_header 3
 str x23,[x19,#16]
 mov x1,#40
 add x2,x19,#512
 mov x3,#296
 bl .Lipc_do_call
 cmp x0,#2
 b.ne .Lipc_fail
.Lipc_current_service_receive:
 ipc_header 2
 ipc_call
 add x9,x19,#512
 ldr x22,[x9,#8]
 ldr x0,[x9,#16]
 cmp x0,#42
 b.ne .Lipc_fail
 ldr x0,[x9,#40]
 mov x1,#0x1234
 cmp x0,x1
 b.ne .Lipc_fail
 ipc_header 3
 str x22,[x19,#16]
 mov x28,#105
 ipc_call
 // Finite effect delay lets the client enter its terminal wait before reply.
 mov x8,#0x20
 lsl x8,x8,#16
.Lipc_service_delay:
 subs x8,x8,#1
 b.ne .Lipc_service_delay
 ipc_header 4
 str x22,[x19,#16]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0x5678
 str x0,[x19,#40]
 mov x28,#106
 ipc_call 48
 ipc_header 8
 mov x28,#107
 ipc_call
.Lipc_pass:
 mov x0,#1
 svc #0x4e
.Lipc_deadline_client:
 // No EL0 deadline polling: wait blocks until the physical timer arbiter wins.
 ipc_header 1
 mov x0,#42
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1
 str x0,[x19,#24]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0x1234
 str x0,[x19,#40]
 mov x28,#110
 ipc_call 48
 mov x22,x1
 ipc_header 5,x22
 mov x28,#111
 ipc_call
 ipc_header 6,x22
 mov x28,#112
 ipc_call
 cmp x1,#24
 b.ne .Lipc_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 mov x1,#6
 cmp x20,#4
 mov x2,#5
 csel x1,x2,x1,eq
 cmp x20,#10
 csel x1,x2,x1,eq
 cmp x20,#6
 mov x2,#4
 csel x1,x2,x1,eq
 cmp x20,#8
 csel x1,x2,x1,eq
 cmp x0,x1
 b.ne .Lipc_fail
 ldr w0,[x9,#16]
 cbnz x0,.Lipc_fail
 cmp x20,#6
 b.hs .Lipc_pass
 // A second real request wakes the blocked receiver and ends the fixture.
 ipc_header 1
 mov x0,#43
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0x1234
 str x0,[x19,#40]
 mov x28,#113
 ipc_call 48
 mov x22,x1
 ipc_header 5,x22
 mov x28,#114
 ipc_call
 ipc_header 6,x22
 mov x28,#115
 ipc_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lipc_fail
 cmp x1,#32
 b.ne .Lipc_fail
 ldr x0,[x9,#24]
 mov x1,#0x5678
 cmp x0,x1
 b.ne .Lipc_fail
 b .Lipc_pass
.Lipc_deadline_service:
 ipc_header 2
 mov x28,#116
 ipc_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 cmp x20,#5
 b.ne .Lipc_deadline_uncommitted
 ipc_header 3
 str x25,[x19,#16]
 mov x28,#117
 ipc_call
.Lipc_deadline_uncommitted:
 // Delivered work remains outstanding while the same service waits for the
 // next queue entry. Both CPUs have no READY task until the request expires.
 ipc_header 2
 mov x28,#118
 ipc_call
 add x9,x19,#512
 ldr x22,[x9,#8]
 ldr x0,[x9,#16]
 cmp x0,#43
 b.ne .Lipc_fail
 ipc_header 3
 str x22,[x19,#16]
 mov x28,#119
 ipc_call
 ipc_header 4
 str x22,[x19,#16]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0x5678
 str x0,[x19,#40]
 mov x28,#120
 ipc_call 48
 ipc_header 8
 mov x28,#121
 ipc_call
 b .Lipc_pass
.Lipc_queued_service_death:
 ipc_header 2
 mov x28,#123
 ipc_bad_copy
 b .Lipc_pass
.Lipc_delivered_service_death:
 ipc_header 2
 mov x28,#124
 ipc_call
 b .Lipc_pass
.Lipc_committed_service_death:
 ipc_header 2
 mov x28,#125
 ipc_call
 add x9,x19,#512
 ldr x22,[x9,#8]
 ipc_header 3
 str x22,[x19,#16]
 mov x28,#126
 ipc_call
 b .Lipc_pass
.Lipc_receive_copy_recovery:
 ipc_header 2
 mov x28,#127
 ipc_bad_copy
 b .Lipc_service
.Lipc_cancel_client:
 mov x23,x22
 ipc_header 1
 mov x0,#42
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 mov x28,#130
 ipc_call
 mov x22,x1
 // The control endpoint is an independent real EL0 request. Receiving it
 // proves the service has delivered/committed work before cancellation.
 ipc_header 2,x23
 mov x28,#131
 ipc_call
 add x9,x19,#512
 ldr x24,[x9,#8]
 ipc_header 7,x22
 mov x28,#132
 ipc_call
 mov x0,#4
 cmp x20,#18
 mov x2,#5
 csel x0,x2,x0,eq
 cmp x1,x0
 b.ne .Lipc_fail
 ipc_header 6,x22
 mov x28,#133
 ipc_call
 cmp x1,#24
 b.ne .Lipc_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 mov x1,#4
 cmp x20,#18
 mov x2,#5
 csel x1,x2,x1,eq
 cmp x0,x1
 b.ne .Lipc_fail
 ipc_header 4,x23
 str x24,[x19,#16]
 mov x28,#134
 ipc_call
 ipc_header 8,x23
 mov x28,#135
 ipc_call
 b .Lipc_pass
.Lipc_cancel_service:
 mov x23,x22
 ipc_header 2
 mov x28,#136
 ipc_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 cmp x20,#19
 b.ne .Lipc_cancel_uncommitted
 ipc_header 3
 str x25,[x19,#16]
 mov x28,#137
 ipc_call
.Lipc_cancel_uncommitted:
 ipc_header 1,x23
 mov x0,#77
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 mov x28,#138
 ipc_call
 mov x26,x1
 ipc_header 5,x26
 mov x28,#139
 ipc_call
 ipc_header 6,x26
 mov x28,#140
 ipc_call
 // Cancellation/result collection invalidates the old service token. A late
 // reply must fail through the actual terminal arbiter, not a helper result.
 ipc_header 4
 str x25,[x19,#16]
 mov x27,#32
.Lipc_cancel_late_reply:
 mov x0,x19
 mov x1,#40
 mov x2,#0
 mov x3,#0
 svc #0xa0
 cmp x0,#14
 b.ne .Lipc_cancel_late_done
 subs x27,x27,#1
 b.ne .Lipc_cancel_late_reply
.Lipc_cancel_late_done:
 mov x28,#141
 cmp x0,#2
 b.ne .Lipc_fail
 ipc_header 8
 mov x28,#142
 ipc_call
 b .Lipc_pass
.Lipc_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lipc_fail
.Lipc_do_call:
 mov x17,x1
 mov x27,#32
.Lipc_do_retry:
 mov x0,x19
 mov x1,x17
 svc #0xa0
 cmp x0,#14
 b.ne .Lipc_do_done
 subs x27,x27,#1
 b.ne .Lipc_do_retry
.Lipc_do_done:
 ret
ipc_image_end:
