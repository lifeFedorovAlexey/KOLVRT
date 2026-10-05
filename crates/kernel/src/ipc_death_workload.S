.section .rodata.ipc_death_image,"a"
.balign 16
.global ipc_death_image_start, ipc_death_image_end
.macro death_header op, selector=x21
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
.macro death_call
 bl .Ldeath_call
 cbnz x0,.Ldeath_fail
.endm
.macro death_submit selector, tag
 death_header 1,\selector
 mov x0,#\tag
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 death_call
.endm
ipc_death_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x28,#150
 cbz x20,.Ldeath_client
 cmp x20,#4
 b.eq .Ldeath_survivor
 // Service roles: 1 delivered, 2 committed, 3 queued. The dying
 // requester owns control endpoint B; its death terminalizes our request.
 cmp x20,#3
 b.eq .Ldeath_control
 death_header 2
 death_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 cmp x20,#2
 b.ne .Ldeath_control
 death_header 3
 str x25,[x19,#16]
 mov x28,#151
 death_call
.Ldeath_control:
 death_submit x22,77
 mov x26,x1
 death_header 5,x26
 mov x28,#152
 death_call
 death_header 6,x26
 death_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cmp x0,#4
 b.ne .Ldeath_fail
 // The control outcome is stamped by actual process death, not a client
 // promise to exit. Only now inspect the old work token or admit new work.
 cmp x20,#3
 b.eq .Ldeath_notify_survivor
 cmp x20,#2
 b.eq .Ldeath_retained_reply
 death_header 3
 str x25,[x19,#16]
 bl .Ldeath_call
 mov x28,#153
 cmp x0,#2
 b.ne .Ldeath_fail
 b .Ldeath_notify_survivor
.Ldeath_retained_reply:
 death_header 4
 str x25,[x19,#16]
 mov x28,#154
 death_call
.Ldeath_notify_survivor:
 death_submit x23,88
 mov x26,x1
 death_header 5,x26
 death_call
 death_header 6,x26
 death_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Ldeath_fail
 death_header 2
 mov x28,#155
 death_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr x0,[x9,#16]
 cmp x0,#43
 b.ne .Ldeath_fail
 death_header 3
 str x25,[x19,#16]
 death_call
 death_header 4
 str x25,[x19,#16]
 death_call
 death_header 8
 death_call
 b .Ldeath_pass
.Ldeath_client:
 death_submit x21,42
 death_header 2,x22
 mov x28,#156
 death_call
 // Exit with a delivered control request unreplied. The kernel must wake
 // the service requester with cancelled-before-effect for endpoint B.
 b .Ldeath_pass
.Ldeath_survivor:
 death_header 2,x22
 mov x28,#157
 death_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 death_header 4,x22
 str x25,[x19,#16]
 death_call
 death_submit x21,43
 mov x26,x1
 death_header 5,x26
 death_call
 death_header 6,x26
 death_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Ldeath_fail
 cmp x1,#24
 b.ne .Ldeath_fail
 death_header 8,x22
 death_call
.Ldeath_pass:
 mov x0,#1
 svc #0x4e
.Ldeath_fail:
 mov x0,x28
 svc #0x4e
 b .Ldeath_fail
.Ldeath_call:
 mov x27,#32
.Ldeath_retry:
 mov x0,x19
 mov x1,#40
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 cmp x0,#14
 b.ne .Ldeath_done
 subs x27,x27,#1
 b.ne .Ldeath_retry
.Ldeath_done:
 ret
ipc_death_image_end:
