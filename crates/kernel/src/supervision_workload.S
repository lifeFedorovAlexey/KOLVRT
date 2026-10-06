.section .rodata.supervision_image,"a"
.balign 16
.global supervisor_image_start, supervisor_image_end
.global supervised_service_start, supervised_service_end
.macro lifecycle op, selector=0, token=xzr, arg=0
 mov x0,#\op
 mov x1,#\selector
 mov x2,\token
 mov x3,#\arg
 mov x4,#1
 svc #0xb0
.endm
.macro sup_header op, selector=x25
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
.macro sup_ipc len=40
 mov x0,x19
 mov x1,#\len
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
.endm
.macro sup_event code
 mov x13,#(1 << (\code - 1))
 orr x12,x12,x13
.endm
supervisor_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x20,#1000
 mov x12,#0
 // Static EL0 manifest: peer first; worker depends on authenticated peer readiness.
 lifecycle 1,1
 cbnz x0,.Lsup_fail
 mov x27,x1
 mov x28,x2
 str x3,[x19,#1024]
 mov x25,x28
 mov x20,#1010
 bl .Lsup_probe
 cbnz x0,.Lsup_fail
 mov x11,#2
 adr x9,.Lsup_worker_definition
 bl .Lsup_launch_definition
 cbnz x0,.Lsup_fail
 mov x24,x1
 mov x25,x2
 mov x20,#1020
 bl .Lsup_probe
 cbnz x0,.Lsup_fail
 orr x11,x11,#1
 mov x20,#1001
 sup_event 1
 // The manifest's missing dependency (readiness bit 4) prevents its dependent start.
 adr x9,.Lsup_missing_definition
 bl .Lsup_launch_definition
 cmp x0,#20
 b.ne .Lsup_dependency_fail
 // Native quota exhaustion must roll back an unpublished child without a completion.
 lifecycle 1,2
 cmp x0,#12
 b.ne .Lsup_fail
 // Forged manifest/grant selector is independently rejected.
 lifecycle 1,99
 cmp x0,#11
 b.ne .Lsup_fail
 sup_event 2
 lifecycle 5
 cbnz x0,.Lsup_fail
 lifecycle 5
 cmp x0,#11
 b.ne .Lsup_fail
 lifecycle 1
 cmp x0,#11
 b.ne .Lsup_fail
 // The never-used launch grant cannot survive sealing.
 lifecycle 1,2
 cmp x0,#11
 b.ne .Lsup_fail
 // Retained service credits are finite and never renewed by sealing.
 lifecycle 3,1,x27
 cmp x0,#12
 b.ne .Lsup_fail
 // An unsupported lifecycle version fails without publication.
 mov x0,#1
 mov x1,#0
 mov x2,#0
 mov x3,#0
 mov x4,#0
 svc #0xb0
 cmp x0,#1
 b.ne .Lsup_fail
 sup_event 3
 // A requester SEND handle cannot fabricate readiness by replying.
 sup_header 4
 mov x0,#1
 str x0,[x19,#16]
 sup_ipc
 cmp x0,#11
 b.ne .Lsup_fail
 sup_event 4
 // Crash after commitment: effect-unknown, then exact death notification.
 mov x20,#1002
 mov x0,#2
 bl .Lsup_submit
 cbnz x0,.Lsup_fail
 bl .Lsup_collect
 cbnz x0,.Lsup_fail
 ldrh w0,[x19,#514]
 cmp x0,#5
 b.ne .Lsup_fail
 bl .Lsup_death
 cmp x1,#2
 b.ne .Lsup_fail
 mov x26,x25
 mov x29,x24
 lifecycle 3,0,x24
 cbnz x0,.Lsup_fail
 mov x24,x1
 mov x25,x2
 cmp x24,x29
 b.eq .Lsup_fail
 lifecycle 2,0,x29
 cmp x0,#2
 b.ne .Lsup_fail
 sup_header 1,x26
 mov x0,#1
 str x0,[x19,#16]
 sup_ipc
 cmp x0,#2
 b.ne .Lsup_fail
 bl .Lsup_probe
 cbnz x0,.Lsup_fail
 sup_event 5
 bl .Lsup_peer_probe
 cbnz x0,.Lsup_fail
 // Readiness timeout is a terminal probe result, never a forged READY flag.
 lifecycle 4,0,x24
 cbnz x0,.Lsup_fail
 bl .Lsup_death
 cmp x1,#3
 b.ne .Lsup_fail
 lifecycle 3,0,x24,1
 cbnz x0,.Lsup_fail
 mov x24,x1
 mov x25,x2
 mov x20,#1003
 mov x0,#1
 bl .Lsup_submit
 cbnz x0,.Lsup_fail
 bl .Lsup_collect
 cbnz x0,.Lsup_fail
 ldrh w0,[x19,#514]
 cmp x0,#6
 b.ne .Lsup_fail
 lifecycle 4,0,x24
 cbnz x0,.Lsup_fail
 bl .Lsup_death
 cmp x1,#3
 b.ne .Lsup_fail
 sup_event 6
 // EL0 owns a three-attempt restart budget and counter-clock backoff.
 mov x26,#3
.Lsup_storm:
 mov x20,#1004
 lifecycle 3,0,x24,2
 cbnz x0,.Lsup_fail
 mov x24,x1
 mov x25,x2
 bl .Lsup_probe
 cbnz x0,.Lsup_fail
 bl .Lsup_death
 cmp x1,#2
 b.ne .Lsup_fail
 svc #0x53
 add x29,x0,x1,lsr #7
.Lsup_backoff:
 svc #0x53
 cmp x0,x29
 b.lo .Lsup_backoff
 subs x26,x26,#1
 b.ne .Lsup_storm
 sup_event 7
 bl .Lsup_peer_probe
 cbnz x0,.Lsup_fail
 // Accepted load is cancelled with its actual commitment outcome.
 mov x25,x28
 mov x0,#3
 bl .Lsup_submit
 cbnz x0,.Lsup_fail
 // Receive a request from this exact service only after its load commit.
 ldr x26,[x19,#1024]
 sup_header 2,x26
 sup_ipc
 cbnz x0,.Lsup_fail
 ldr x29,[x19,#520]
 ldr x0,[x19,#552]
 mov x1,#0xc017
 cmp x0,x1
 b.ne .Lsup_fail
 sup_header 3,x26
 str x29,[x19,#16]
 sup_ipc
 cbnz x0,.Lsup_fail
 sup_header 4,x26
 str x29,[x19,#16]
 sup_ipc
 cbnz x0,.Lsup_fail
 sup_header 7,x22
 sup_ipc
 cbnz x0,.Lsup_fail
 bl .Lsup_collect
 cbnz x0,.Lsup_fail
 ldrh w0,[x19,#514]
 cmp x0,#5
 b.ne .Lsup_unproven_commit
 lifecycle 4,1,x27
 cbnz x0,.Lsup_fail
 mov x24,x27
 bl .Lsup_peer_death
 cmp x1,#3
 b.ne .Lsup_fail
 sup_event 8
 mov x0,#1
 svc #0x4e
.Lsup_dependency_fail:
 mov x29,x0
 mov x0,#1200
 svc #0x4e
.Lsup_unproven_commit:
 mov x29,x0
 mov x0,#1100
 svc #0x4e
.Lsup_fail:
 mov x29,x0
 mov x9,x1
 svc #0x53
 mov x5,x0
 mov x1,x9
 mov x0,x20
 svc #0x4e
.Lsup_submit:
 mov x18,x30
 mov x10,#1
 mov x17,x0
 sup_header 1
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 str x17,[x19,#40]
 add x0,x20,x17
 str x0,[x19,#16]
 // Readiness probes keep their policy deadline. The cancellation scenario
 // selects the maximum valid absolute deadline so expiry cannot substitute
 // for the explicit Cancel under test (zero is already expired by the ABI).
 mov x0,#-1
 mov x6,x0
 mov x7,#0
 str x0,[x19,#24]
 cmp x17,#3
 b.eq .Lsup_submit_call
 svc #0x53
 mov x7,x0
 mov x8,x1
 add x0,x0,x1,lsr #3
 mov x6,x0
 str x0,[x19,#24]
.Lsup_submit_call:
 sup_ipc 48
 mov x22,x1
 mov x30,x18
 ret
.Lsup_collect:
 mov x18,x30
 mov x10,#2
 sup_header 5,x22
 sup_ipc
 cbnz x0,.Lsup_collect_return
 mov x10,#3
 sup_header 6,x22
 sup_ipc
.Lsup_collect_return:
 mov x30,x18
 ret
.Lsup_probe:
 mov x16,x30
 mov x0,#1
 bl .Lsup_submit
 cbnz x0,.Lsup_probe_return
 bl .Lsup_collect
 cbnz x0,.Lsup_probe_return
 mov x10,#4
 ldrh w0,[x19,#514]
 cbnz x0,.Lsup_probe_return
 ldr x0,[x19,#536]
 mov x1,#0x5244
 cmp x0,x1
 cset x0,ne
.Lsup_probe_return:
 mov x30,x16
 ret
.Lsup_peer_probe:
 mov x15,x30
 mov x14,x25
 mov x25,x28
 bl .Lsup_probe
 mov x25,x14
 mov x30,x15
 ret
.Lsup_death:
 lifecycle 2,0,x24
 cbnz x0,.Lsup_fail
 cbz x1,.Lsup_death
 ret
.Lsup_peer_death:
 lifecycle 2,1,x24
 cbnz x0,.Lsup_fail
 cbz x1,.Lsup_peer_death
 ret
// The same EL0 policy path handles a launch request with satisfied or absent dependencies.
.Lsup_launch_definition:
 ldr x8,[x9,#8]
 and x10,x8,x11
 cmp x10,x8
 .if !{dependency_negative}
 b.ne .Lsup_dependency_denied
 .endif
 mov x0,#1
 ldr x1,[x9]
 mov x2,#0
 mov x3,#0
 mov x4,#1
 svc #0xb0
 ret
.Lsup_dependency_denied:
 mov x0,#20
 mov x1,#0
 mov x2,#0
 mov x3,#0
 mov x4,#0
 ret
.balign 8
.Lsup_worker_definition:
 .quad 0,2,3,128
.Lsup_missing_definition:
 .quad 2,4,3,128 // grant selector, dependency readiness mask, restart budget, backoff divisor
supervisor_image_end:
.balign 16
supervised_service_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x25,x21
 mov x27,x22
 // A child cannot acquire lifecycle authority using the supervisor's selector.
 lifecycle 1
 cmp x0,#11
 b.ne .Lservice_fail
 mov x24,#0
.Lservice_receive:
 sup_header 2
 sup_ipc
 cbnz x0,.Lservice_fail
 ldr x22,[x19,#520]
 ldr x26,[x19,#552]
 cmp x23,#1
 b.eq .Lservice_silent
 .if {commit_negative}
 // Mutation: forge the commit acknowledgement before the load is committed.
 cmp x26,#3
 b.eq .Lservice_committed_load
 .endif
 sup_header 3
 str x22,[x19,#16]
 sup_ipc
 cbnz x0,.Lservice_fail
 cmp x26,#2
 b.eq .Lservice_crash
 cmp x26,#3
 b.eq .Lservice_committed_load
 sup_header 4
 str x22,[x19,#16]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0x5244
 str x0,[x19,#40]
 sup_ipc 48
 cbnz x0,.Lservice_fail
 add x24,x24,#1
 cmp x23,#2
 b.eq .Lservice_crash
 b .Lservice_receive
.Lservice_committed_load:
 sup_header 1,x27
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 mov x0,#0xc017
 str x0,[x19,#40]
 mov x0,#1
 str x0,[x19,#16]
 // This acknowledgement proves actual commit for the cancellation test.
 // Select the maximum valid absolute deadline for this handshake as well.
 mov x0,#-1
 str x0,[x19,#24]
 sup_ipc 48
 cbnz x0,.Lservice_fail
 mov x28,x1
 sup_header 5,x28
 sup_ipc
 cbnz x0,.Lservice_fail
 sup_header 6,x28
 sup_ipc
 cbnz x0,.Lservice_fail
 b .Lservice_silent
.Lservice_silent:
 b .Lservice_silent
.Lservice_crash:
 .inst 0x00000000
.Lservice_fail:
 mov x0,#900
 svc #0x4e
supervised_service_end:
