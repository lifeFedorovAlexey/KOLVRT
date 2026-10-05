.section .rodata.ipc_revoke_image,"a"
.balign 16
.global ipc_revoke_image_start,ipc_revoke_image_end
.macro revoke_header op,selector=x21
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
.macro revoke_call length=40
 mov x1,#\length
 bl .Lrevoke_call
 cbnz x0,.Lrevoke_fail
.endm
.macro revoke_submit selector,tag
 revoke_header 1,\selector
 str \tag,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
.endm
ipc_revoke_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x18,#0
 mov x16,#0
 mov x17,#0
 mov x28,#253
 cmp x20,#2
 b.eq .Lrevoke_service
.Lrevoke_attempt:
 cbnz x20,.Lrevoke_send
 cmp x18,#2
 b.ne .Lrevoke_send
 // Revocation races the other CPU's bounded admission attempts. It must
 // close the shared gate without destroying any earlier accepted receipt.
 mov x0,#1
 str x0,[x19]
 mov x0,#4
 str x0,[x19,#8]
 str x21,[x19,#16]
 stp xzr,xzr,[x19,#24]
 str xzr,[x19,#40]
 mov x0,x19
 svc #0x80
 cbnz x0,.Lrevoke_fail
.Lrevoke_send:
 add x24,x18,#1
 add x24,x24,x20,lsl #8
 revoke_submit x21,x24
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 str x24,[x19,#40]
 mov x1,#48
 bl .Lrevoke_call
 cbz x0,.Lrevoke_accepted
 cmp x0,#11
 b.eq .Lrevoke_denied
 cmp x0,#12
 b.ne .Lrevoke_fail
 // Full admission is explicit and retains no receipt or charge.
 b .Lrevoke_next_attempt
.Lrevoke_denied:
 add x17,x17,#1
 b .Lrevoke_next_attempt
.Lrevoke_accepted:
 cbnz x20,.Lrevoke_save
 cmp x18,#2
 b.hs .Lrevoke_fail
.Lrevoke_save:
 add x9,x19,#1024
 str x1,[x9,x16,lsl #3]
 add x9,x19,#1152
 str x24,[x9,x16,lsl #3]
 add x16,x16,#1
.Lrevoke_next_attempt:
 add x18,x18,#1
 cmp x18,#16
 b.ne .Lrevoke_attempt
 cbnz x20,.Lrevoke_done
 cmp x17,#14
 b.ne .Lrevoke_fail
.Lrevoke_done:
 add x24,x20,#77
 revoke_submit x22,x24
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 str x16,[x19,#40]
 revoke_call 48
 mov x25,x1
 revoke_header 5,x25
 revoke_call
 revoke_header 6,x25
 revoke_call
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lrevoke_fail
 mov x18,#0
.Lrevoke_collect:
 cmp x18,x16
 b.eq .Lrevoke_pass
 add x9,x19,#1024
 ldr x25,[x9,x18,lsl #3]
 revoke_header 5,x25
 revoke_call
 revoke_header 6,x25
 revoke_call
 cmp x1,#32
 b.ne .Lrevoke_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lrevoke_fail
 ldr x0,[x9,#24]
 add x9,x19,#1152
 ldr x1,[x9,x18,lsl #3]
 cmp x0,x1
 b.ne .Lrevoke_fail
 add x18,x18,#1
 b .Lrevoke_collect
.Lrevoke_service:
 // Clients report exact accepted counts over a separately scoped endpoint.
 // Only after both reports is the finite main queue drained and completed.
 revoke_header 2,x22
 revoke_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr x0,[x9,#40]
 cmp x0,#4
 b.hi .Lrevoke_fail
 add x16,x16,x0
 add x9,x19,#1024
 str x25,[x9,x18,lsl #3]
 add x18,x18,#1
 cmp x18,#2
 b.ne .Lrevoke_service
 cmp x16,#4
 b.hi .Lrevoke_fail
 mov x18,#0
.Lrevoke_drain:
 cmp x18,x16
 b.eq .Lrevoke_reply_controls
 revoke_header 2
 revoke_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr x24,[x9,#16]
 ldr x0,[x9,#40]
 cmp x0,x24
 b.ne .Lrevoke_fail
 revoke_header 3
 str x25,[x19,#16]
 revoke_call
 revoke_header 4
 str x25,[x19,#16]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 str x24,[x19,#40]
 revoke_call 48
 add x18,x18,#1
 b .Lrevoke_drain
.Lrevoke_reply_controls:
 revoke_header 8
 revoke_call
 mov x18,#0
.Lrevoke_control_reply:
 add x9,x19,#1024
 ldr x25,[x9,x18,lsl #3]
 revoke_header 4,x22
 str x25,[x19,#16]
 revoke_call
 add x18,x18,#1
 cmp x18,#2
 b.ne .Lrevoke_control_reply
 revoke_header 8,x22
 revoke_call
.Lrevoke_pass:
 mov x0,#1
 svc #0x4e
.Lrevoke_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lrevoke_fail
.Lrevoke_call:
 mov x0,x19
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 ret
ipc_revoke_image_end:
