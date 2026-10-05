.section .rodata.ipc_queue_image,"a"
.balign 16
.global ipc_queue_image_start,ipc_queue_image_end
.macro queue_header op,selector=x21
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
.macro queue_call expected=0
 bl .Lqueue_call
 cmp x0,#\expected
 b.ne .Lqueue_fail
.endm
.macro queue_submit selector,tag
 queue_header 1,\selector
 str \tag,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
.endm
ipc_queue_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 add x18,x19,#1024
 mov x28,#230
 cmp x20,#9
 b.eq .Lqueue_pass
 tst x20,#1
 b.ne .Lqueue_service
 queue_header 2,x22
 queue_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 mov x26,#0
.Lqueue_fill:
 add x24,x26,#100
 queue_submit x21,x24
 queue_call
 str x1,[x18,x26,lsl #3]
 add x26,x26,#1
 cmp x26,x23
 b.ne .Lqueue_fill
 mov x24,#999
 queue_submit x21,x24
 queue_call 12
 cmp x20,#2
 b.ne .Lqueue_unblock_service
 // The service is held behind its control request: both cancellations
 // linearize while these exact head/non-head requests remain queued.
 ldr x24,[x18]
 queue_header 7,x24
 queue_call
 cmp x1,#4
 b.ne .Lqueue_fail
 ldr x24,[x18,#16]
 queue_header 7,x24
 queue_call
 cmp x1,#4
 b.ne .Lqueue_fail
.Lqueue_unblock_service:
 queue_header 4,x22
 str x25,[x19,#16]
 queue_call
 cmp x20,#4
 b.eq .Lqueue_pass
 mov x26,#0
.Lqueue_collect:
 ldr x24,[x18,x26,lsl #3]
 queue_header 5,x24
 queue_call
 queue_header 6,x24
 queue_call
 cmp x1,#24
 b.ne .Lqueue_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 mov x1,#0
 cmp x20,#6
 b.ne .Lqueue_cancel_outcome
 mov x1,#4
 b .Lqueue_check_outcome
.Lqueue_cancel_outcome:
 cmp x20,#2
 b.ne .Lqueue_check_outcome
 tst x26,#1
 mov x2,#4
 csel x1,x2,x1,eq
.Lqueue_check_outcome:
 cmp x0,x1
 b.ne .Lqueue_fail
 ldr x0,[x9,#8]
 add x1,x26,#100
 cmp x0,x1
 b.ne .Lqueue_fail
 add x26,x26,#1
 cmp x26,x23
 b.ne .Lqueue_collect
 queue_header 8,x22
 queue_call
 b .Lqueue_pass
.Lqueue_service:
 mov x24,#77
 queue_submit x22,x24
 queue_call
 mov x25,x1
 queue_header 5,x25
 queue_call
 queue_header 6,x25
 queue_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lqueue_fail
 cmp x20,#5
 b.eq .Lqueue_shutdown
 cmp x20,#7
 b.eq .Lqueue_shutdown
 mov x26,#0
.Lqueue_drain:
 queue_header 2
 queue_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr x0,[x9,#16]
 add x1,x26,#100
 cmp x20,#3
 b.ne .Lqueue_check_id
 lsl x1,x26,#1
 add x1,x1,#101
.Lqueue_check_id:
 cmp x0,x1
 b.ne .Lqueue_fail
 queue_header 3
 str x25,[x19,#16]
 queue_call
 queue_header 4
 str x25,[x19,#16]
 queue_call
 add x26,x26,#1
 mov x0,x23
 cmp x20,#3
 mov x1,#2
 csel x0,x1,x0,eq
 cmp x26,x0
 b.ne .Lqueue_drain
.Lqueue_shutdown:
 cmp x20,#7
 b.ne .Lqueue_shutdown_ready
 // Make shutdown race an actual blocked requester. Finite clock-bounded
 // computation gives its peer a slice; the kernel fixture verifies blocking.
 svc #0x53
 mov x9,#200
 udiv x13,x1,x9
 add x13,x0,x13
.Lqueue_shutdown_delay:
 mov x8,#65536
1:
 subs x8,x8,#1
 b.ne 1b
 svc #0x53
 cmp x0,x13
 b.lo .Lqueue_shutdown_delay
.Lqueue_shutdown_ready:
 queue_header 8
 queue_call
.Lqueue_pass:
 mov x0,#1
 svc #0x4e
.Lqueue_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lqueue_fail
.Lqueue_call:
 mov x0,x19
 mov x1,#40
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 ret
ipc_queue_image_end:
