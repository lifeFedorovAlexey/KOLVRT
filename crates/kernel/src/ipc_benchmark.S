.section .rodata.ipc_benchmark_image,"a"
.balign 16
.global ipc_benchmark_image_start,ipc_benchmark_image_end
.macro bench_header op
 mov x0,#1
 strh w0,[x19]
 mov x0,#\op
 strh w0,[x19,#2]
 mov x0,#40
 str w0,[x19,#4]
 str x21,[x19,#8]
 stp xzr,xzr,[x19,#16]
 str xzr,[x19,#32]
.endm
.macro bench_call length=40
 mov x1,#\length
 bl .Lbench_call
 cbnz x0,.Lbench_fail
.endm
.macro bench_start kind
 cmp x24,#\kind
 b.ne 1f
 svc #0x53
 mov x27,x0
1:
.endm
.macro bench_end kind
 cmp x24,#\kind
 b.ne 1f
 svc #0x53
 sub x0,x0,x27
 svc #0x54
 cbnz x0,.Lbench_fail
1:
.endm
ipc_benchmark_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x18,#0
 mov x28,#250
.Lbench_round:
 cbnz x20,.Lbench_service
 bench_header 1
 mov x0,#42
 str x0,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
 add x1,x23,#40
 str w1,[x19,#4]
 str w23,[x19,#32]
 add x9,x19,#40
 mov x10,#0
.Lbench_request_fill:
 cmp x10,x23
 b.hs .Lbench_request_ready
 strb w18,[x9,x10]
 add x10,x10,#1
 b .Lbench_request_fill
.Lbench_request_ready:
 bench_start 1
 bench_start 3
 add x1,x23,#40
 bl .Lbench_call
 cbnz x0,.Lbench_fail
 mov x25,x1
 bench_end 1
 bench_header 5
 str x25,[x19,#8]
 bench_call
 bench_header 6
 str x25,[x19,#8]
 bench_start 2
 bench_call
 add x0,x23,#24
 cmp x1,x0
 b.ne .Lbench_fail
 bench_end 2
 bench_end 3
 b .Lbench_next
.Lbench_service:
 bench_header 2
 bench_start 4
 bench_call
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr w0,[x9,#32]
 cmp x0,x23
 b.ne .Lbench_fail
 bench_end 4
 bench_header 3
 str x25,[x19,#16]
 bench_call
 bench_header 4
 str x25,[x19,#16]
 add x1,x23,#40
 str w1,[x19,#4]
 str w23,[x19,#32]
 add x9,x19,#40
 mov x10,#0
.Lbench_response_fill:
 cmp x10,x23
 b.hs .Lbench_response_ready
 strb w18,[x9,x10]
 add x10,x10,#1
 b .Lbench_response_fill
.Lbench_response_ready:
 bench_start 5
 add x1,x23,#40
 bl .Lbench_call
 cbnz x0,.Lbench_fail
 bench_end 5
.Lbench_next:
 add x18,x18,#1
 cmp x18,#20
 b.ne .Lbench_round
 cbz x20,.Lbench_pass
 bench_header 8
 bench_call
.Lbench_pass:
 mov x0,#1
 svc #0x4e
.Lbench_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lbench_fail
.Lbench_call:
 mov x0,x19
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 ret
ipc_benchmark_image_end:
