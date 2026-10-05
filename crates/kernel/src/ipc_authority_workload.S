.section .rodata.ipc_authority_image,"a"
.balign 16
.global ipc_authority_image_start, ipc_authority_image_end
.macro auth_header op, selector=x21
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
.macro auth_expect code
 bl .Lauth_call
 cmp x0,#\code
 b.ne .Lauth_fail
 add x28,x28,#1
.endm
.macro auth_submit selector, tag
 auth_header 1,\selector
 mov x0,#\tag
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
.endm
ipc_authority_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x28,#180
 cmp x20,#9
 b.eq .Lauth_pass
 cmp x20,#2
 b.eq .Lauth_request_quota
 cbnz x20,.Lauth_service
 auth_header 1,xzr
 auth_expect 1
 auth_header 1,x23
 auth_expect 3
 auth_submit x22,99
 auth_expect 9
 auth_header 2
 auth_expect 11
 auth_header 8
 auth_expect 11
 auth_header 1
 strh wzr,[x19]
 auth_expect 13
 auth_header 99
 auth_expect 13
 auth_header 1
 mov x0,#1
 str w0,[x19,#36]
 auth_expect 1
 auth_header 3
 mov x0,#1
 str x0,[x19,#16]
 auth_expect 11
 auth_header 4
 mov x0,#1
 str x0,[x19,#16]
 auth_expect 11
 // Unknown rights through the real handle boundary reject before effects.
 mov x0,#1
 str x0,[x19]
 str x0,[x19,#8]
 str x21,[x19,#16]
 mov x0,#3
 str x0,[x19,#24]
 mov x0,#8
 str x0,[x19,#32]
 str xzr,[x19,#40]
 mov x0,x19
 svc #0x80
 cmp x0,#9
 b.ne .Lauth_fail
 add x28,x28,#1
 auth_submit x21,42
 auth_expect 0
 mov x26,x1
 // Service waits for our real control reply before replying to the accepted
 // work. This keeps it alive while future SEND is revoked and the alias closed.
 auth_header 2,x24
 auth_expect 0
 add x9,x19,#512
 ldr x25,[x9,#8]
 mov x0,#1
 str x0,[x19]
 mov x0,#4
 str x0,[x19,#8]
 str x21,[x19,#16]
 stp xzr,xzr,[x19,#24]
 str xzr,[x19,#40]
 mov x0,x19
 svc #0x80
 cbnz x0,.Lauth_fail
 auth_submit x21,43
 auth_expect 11
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
 cbnz x0,.Lauth_fail
 auth_header 1
 auth_expect 2
 auth_header 4,x24
 str x25,[x19,#16]
 auth_expect 0
 auth_header 5,x26
 auth_expect 0
 auth_header 6,x26
 auth_expect 0
 cmp x1,#24
 b.ne .Lauth_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lauth_fail
 auth_header 8,x24
 auth_expect 0
 b .Lauth_pass
.Lauth_request_quota:
 auth_submit x21,44
 auth_expect 12
 b .Lauth_pass
.Lauth_service:
 auth_header 2
 auth_expect 0
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr x0,[x9,#16]
 cmp x0,#42
 b.ne .Lauth_fail
 auth_header 3
 str x25,[x19,#16]
 auth_expect 0
 auth_submit x22,77
 auth_expect 0
 mov x26,x1
 auth_header 5,x26
 auth_expect 0
 auth_header 6,x26
 auth_expect 0
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lauth_fail
 auth_header 4
 str x25,[x19,#16]
 auth_expect 0
 auth_header 8
 auth_expect 0
.Lauth_pass:
 mov x0,#1
 svc #0x4e
.Lauth_fail:
 mov x0,x28
 svc #0x4e
 b .Lauth_fail
.Lauth_call:
 mov x27,#32
.Lauth_retry:
 mov x0,x19
 mov x1,#40
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 cmp x0,#14
 b.ne .Lauth_done
 subs x27,x27,#1
 b.ne .Lauth_retry
.Lauth_done:
 ret
ipc_authority_image_end:
