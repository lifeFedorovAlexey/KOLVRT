.section .rodata.ipc_multi_image,"a"
.balign 16
.global ipc_multi_image_start,ipc_multi_image_end
.macro multi_header op,selector=x21
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
.macro multi_call length=40
 mov x1,#\length
 bl .Lmulti_call
 cbnz x0,.Lmulti_fail
.endm
ipc_multi_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x18,#0
 mov x28,#252
 cmp x20,#2
 b.eq .Lmulti_service
.Lmulti_client:
 multi_header 1
 add x24,x18,#1
 add x24,x24,x20,lsl #8
 str x24,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 str x24,[x19,#40]
 multi_call 48
 mov x25,x1
 multi_header 5,x25
 multi_call
 multi_header 6,x25
 multi_call
 cmp x1,#32
 b.ne .Lmulti_fail
 add x9,x19,#512
 ldrh w0,[x9,#2]
 cbnz x0,.Lmulti_fail
 ldr x0,[x9,#8]
 cmp x0,x24
 b.ne .Lmulti_fail
 ldr x0,[x9,#24]
 cmp x0,x24
 b.ne .Lmulti_fail
 add x18,x18,#1
 cmp x18,#16
 b.ne .Lmulti_client
 b .Lmulti_pass
.Lmulti_service:
 mov x26,#0
.Lmulti_receive:
 multi_header 2
 multi_call
 cmp x1,#48
 b.ne .Lmulti_fail
 add x9,x19,#512
 ldr x25,[x9,#8]
 // Request generation is allocated at successful endpoint admission.
 // Its increasing order proves FIFO for concurrent producers on that cell.
 cmp x25,x26
 b.ls .Lmulti_fail
 mov x26,x25
 ldr x24,[x9,#16]
 ldr x0,[x9,#40]
 cmp x0,x24
 b.ne .Lmulti_fail
 multi_header 3
 str x25,[x19,#16]
 multi_call
 multi_header 4
 str x25,[x19,#16]
 mov x0,#48
 str w0,[x19,#4]
 mov x0,#8
 str w0,[x19,#32]
 str x24,[x19,#40]
 multi_call 48
 add x18,x18,#1
 cmp x18,#32
 b.ne .Lmulti_receive
 multi_header 8
 multi_call
.Lmulti_pass:
 mov x0,#1
 svc #0x4e
.Lmulti_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lmulti_fail
.Lmulti_call:
 mov x0,x19
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 ret
ipc_multi_image_end:
