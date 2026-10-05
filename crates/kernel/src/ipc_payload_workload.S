.section .rodata.ipc_payload_image,"a"
.balign 16
.global ipc_payload_image_start, ipc_payload_image_end
.macro payload_header op, selector=x21
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
.macro payload_call expected=0, length=40
 mov x1,#\length
 add x2,x19,#512
 bl .Lpayload_call
 cmp x0,#\expected
 b.ne .Lpayload_fail
.endm
.macro payload_submit selector, tag
 payload_header 1,\selector
 mov x0,#\tag
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
.endm
ipc_payload_image_start:
 // Every transport snapshot is unaligned and spans two writable user pages.
 sub x19,sp,#2,lsl #12
 sub x19,x19,#17
 mov x26,#0
 mov x18,#0
 mov x28,#210
 cbnz x20,.Lpayload_round
 payload_header 1
 strh wzr,[x19]
 payload_call 13
 payload_header 99
 payload_call 13
 payload_header 1
 mov x0,#1
 str w0,[x19,#36]
 payload_call 1
 payload_header 1
 mov x0,#41
 str w0,[x19,#4]
 payload_call 1
 payload_header 1
 mov x0,#257
 str w0,[x19,#32]
 payload_call 1
 payload_header 1
 mov w0,#-1
 str w0,[x19,#4]
 str w0,[x19,#32]
 payload_call 1
 payload_header 1
 mov x0,#297
 str w0,[x19,#4]
 mov x0,#257
 str w0,[x19,#32]
 payload_call 8,297
 mov x0,#-16
 mov x1,#40
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 cmp x0,#8
 b.ne .Lpayload_fail
.Lpayload_round:
 adr x9,.Lpayload_sizes
 ldrh w16,[x9,x18,lsl #1]
 tst x20,#1
 b.ne .Lpayload_service
 // Fill and submit an exact-sized immutable request snapshot.
 payload_submit x21,42
 add x1,x16,#40
 str w1,[x19,#4]
 str w16,[x19,#32]
 add x9,x19,#40
 mov x10,#0
.Lpayload_fill_request:
 cmp x10,x16
 b.hs .Lpayload_submit_ready
 strb w26,[x9,x10]
 add x10,x10,#1
 b .Lpayload_fill_request
.Lpayload_submit_ready:
 add x2,x19,#512
 bl .Lpayload_call
 cbnz x0,.Lpayload_fail
 mov x17,x25
 mov x25,x1
 mov x0,#255
 add x9,x19,#40
 mov x10,#0
.Lpayload_mutate_request:
 cmp x10,x16
 b.hs .Lpayload_control_receive
 strb w0,[x9,x10]
 add x10,x10,#1
 b .Lpayload_mutate_request
.Lpayload_control_receive:
 cbz x26,.Lpayload_current_receipt
 // The old receipt must never select the newly admitted reused request slot.
 payload_header 7,x17
 payload_call 2
.Lpayload_current_receipt:
 payload_header 2,x22
 payload_call
 add x9,x19,#512
 ldr x24,[x9,#8]
 cmp x20,#2
 b.eq .Lpayload_pass
 payload_header 7,x25
 payload_call 15
 // Control delivery proves original work has reached terminal state. Its
 // client ID must still be reserved until successful collection.
 payload_submit x21,42
 payload_call 14
 cbnz x26,.Lpayload_collect_retry
 payload_header 6,x25
 mov x0,x19
 mov x1,#40
 mov x2,#0x20000000
 mov x3,#296
 svc #0xa0
 cmp x0,#8
 b.ne .Lpayload_fail
 payload_header 6,x25
 mov x0,x19
 mov x1,#40
 add x2,x19,#512
 mov x3,#23
 svc #0xa0
 cmp x0,#1
 b.ne .Lpayload_fail
 payload_header 6,x25
 mov x0,x19
 mov x1,#40
 mov x2,#-16
 mov x3,#296
 svc #0xa0
 cmp x0,#8
 b.ne .Lpayload_fail
.Lpayload_collect_retry:
 payload_header 6,x25
 mov x1,#40
 mov x2,#0
 bl .Lpayload_call
 cmp x0,#8
 b.ne .Lpayload_fail
 payload_submit x21,42
 payload_call 14
 payload_header 6,x25
 payload_call
 add x0,x16,#24
 cmp x1,x0
 b.ne .Lpayload_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lpayload_fail
 ldr w0,[x9,#16]
 cmp x0,x16
 b.ne .Lpayload_fail
 ldr w0,[x9,#20]
 cbnz x0,.Lpayload_fail
 add x9,x9,#24
 mov x11,#0x5a
 eor x11,x26,x11
 mov x10,#0
.Lpayload_check_response:
 cmp x10,x16
 b.hs .Lpayload_control_reply
 ldrb w0,[x9,x10]
 cmp x0,x11
 b.ne .Lpayload_fail
 add x10,x10,#1
 b .Lpayload_check_response
.Lpayload_control_reply:
 payload_header 4,x22
 str x24,[x19,#16]
 payload_call
 b .Lpayload_next
.Lpayload_service:
 payload_header 2
 cbnz x26,.Lpayload_receive_retry
 mov x0,x19
 mov x1,#40
 mov x2,#0x20000000
 mov x3,#296
 svc #0xa0
 cmp x0,#8
 b.ne .Lpayload_fail
 payload_header 2
 mov x0,x19
 mov x1,#40
 add x2,x19,#512
 mov x3,#39
 svc #0xa0
 cmp x0,#1
 b.ne .Lpayload_fail
 payload_header 2
 mov x0,x19
 mov x1,#40
 mov x2,#-16
 mov x3,#296
 svc #0xa0
 cmp x0,#8
 b.ne .Lpayload_fail
.Lpayload_receive_retry:
 payload_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr w0,[x9,#32]
 cmp x0,x16
 b.ne .Lpayload_fail
 ldr w0,[x9,#36]
 cbnz x0,.Lpayload_fail
 add x9,x9,#40
 mov x10,#0
.Lpayload_check_request:
 cmp x10,x16
 b.hs .Lpayload_commit
 ldrb w0,[x9,x10]
 cmp x0,x26
 b.ne .Lpayload_fail
 add x10,x10,#1
 b .Lpayload_check_request
.Lpayload_commit:
 payload_header 3
 str x25,[x19,#16]
 payload_call
 payload_header 4
 str x25,[x19,#16]
 add x1,x16,#40
 str w1,[x19,#4]
 str w16,[x19,#32]
 add x9,x19,#40
 mov x11,#0x5a
 eor x11,x26,x11
 mov x10,#0
.Lpayload_fill_response:
 cmp x10,x16
 b.hs .Lpayload_reply_ready
 strb w11,[x9,x10]
 add x10,x10,#1
 b .Lpayload_fill_response
.Lpayload_reply_ready:
 add x2,x19,#512
 bl .Lpayload_call
 cbnz x0,.Lpayload_fail
 add x9,x19,#40
 mov x0,#255
 mov x10,#0
.Lpayload_mutate_response:
 cmp x10,x16
 b.hs .Lpayload_control_send
 strb w0,[x9,x10]
 add x10,x10,#1
 b .Lpayload_mutate_response
.Lpayload_control_send:
 // The just-completed token is stale even before its requester consumes it.
 payload_header 3
 str x25,[x19,#16]
 payload_call 2
 payload_submit x22,77
 payload_call
 mov x24,x1
 payload_header 5,x24
 payload_call
 payload_header 6,x24
 payload_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cmp x20,#3
 b.ne .Lpayload_control_completed
 cmp x0,#4
 b.ne .Lpayload_fail
 b .Lpayload_finish
.Lpayload_control_completed:
 cbnz x0,.Lpayload_fail
.Lpayload_next:
 add x26,x26,#1
 add x18,x18,#1
 cmp x18,#6
 b.ne .Lpayload_size_ready
 mov x18,#0
.Lpayload_size_ready:
 cmp x26,#24
 b.ne .Lpayload_round
.Lpayload_finish:
 mov x23,x21
 cbnz x20,.Lpayload_shutdown
 mov x23,x22
.Lpayload_shutdown:
 payload_header 8,x23
 payload_call
.Lpayload_pass:
 mov x0,#1
 svc #0x4e
.Lpayload_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lpayload_fail
.Lpayload_call:
 mov x0,x19
 mov x3,#296
 svc #0xa0
 ret
.balign 2
.Lpayload_sizes:
 .hword 0,1,8,64,255,256
ipc_payload_image_end:
