.section .rodata.ipc_paths_benchmark_image,"a"
.balign 16
.global ipc_paths_benchmark_image_start,ipc_paths_benchmark_image_end
.macro path_header op,selector=x21
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
.macro path_call
 mov x1,#40
 bl .Lpath_call
 cbnz x0,.Lpath_fail
.endm
.macro path_submit selector,tag
 path_header 1,\selector
 str \tag,[x19,#16]
 svc #0x53
 add x0,x0,x1,lsl #2
 str x0,[x19,#24]
.endm
.macro path_payload
 add x1,x23,#40
 str w1,[x19,#4]
 str w23,[x19,#32]
 add x9,x19,#40
 mov x10,#0
1:
 cmp x10,x23
 b.hs 2f
 strb w18,[x9,x10]
 add x10,x10,#1
 b 1b
2:
.endm
.macro path_start
 svc #0x53
 mov x27,x0
 mov x26,x4
.endm
.macro path_end blocks
 svc #0x53
 sub x4,x4,x26
 cmp x4,#\blocks
 b.ne .Lpath_fail
 sub x0,x0,x27
 svc #0x54
 cbnz x0,.Lpath_fail
.endm
.macro path_delay
 // Finite EL0 computation allows the peer to block. CLOCK counter deltas
 // below independently reject the group unless the measured call did block.
 svc #0x53
 mov x9,#200
 udiv x12,x1,x9
 add x12,x0,x12
1:
 mov x8,#65536
2:
 subs x8,x8,#1
 b.ne 2b
 svc #0x53
 cmp x0,x12
 b.lo 1b
.endm
ipc_paths_benchmark_image_start:
 mov x19,#0x20000000
 add x19,x19,#4096
 mov x18,#0
 mov x28,#251
.Lpath_round:
 cbnz x20,.Lpath_service
 cmp x24,#6
 b.eq .Lpath_client_gate
 cmp x24,#9
 b.eq .Lpath_client_gate
 cmp x24,#7
 b.ne .Lpath_client_submit
 path_delay
 b .Lpath_client_submit
.Lpath_client_gate:
 // Service waits on this reverse request. Reply only after main work is
 // queued, so its main receive cannot depend on timing or scheduler order.
 path_header 2,x22
 path_call
 add x9,x19,#512
 ldr x15,[x9,#8]
 cmp x24,#9
 b.eq .Lpath_client_full
.Lpath_client_submit:
 mov x14,#42
 path_submit x21,x14
 path_payload
 add x1,x23,#40
 bl .Lpath_call
 cbnz x0,.Lpath_fail
 mov x25,x1
 cmp x24,#6
 b.ne .Lpath_client_wait
 path_header 4,x22
 str x15,[x19,#16]
 path_call
.Lpath_client_wait:
 path_header 5,x25
 cmp x24,#8
 b.ne 1f
 path_start
1:
 path_call
 cmp x24,#8
 b.ne 2f
 path_end 1
2:
 path_header 6,x25
 path_call
 add x0,x23,#24
 cmp x1,x0
 b.ne .Lpath_fail
 b .Lpath_next
.Lpath_client_full:
 mov x16,#0
.Lpath_client_fill:
 add x14,x16,#100
 path_submit x21,x14
 path_payload
 add x1,x23,#40
 bl .Lpath_call
 cbnz x0,.Lpath_fail
 add x9,x19,#1024
 str x1,[x9,x16,lsl #3]
 add x16,x16,#1
 cmp x16,#4
 b.ne .Lpath_client_fill
 mov x14,#999
 path_submit x21,x14
 path_payload
 path_start
 add x1,x23,#40
 bl .Lpath_call
 cmp x0,#12
 b.ne .Lpath_fail
 path_end 0
 path_header 4,x22
 str x15,[x19,#16]
 path_call
 mov x16,#0
.Lpath_client_drain:
 add x9,x19,#1024
 ldr x25,[x9,x16,lsl #3]
 path_header 5,x25
 path_call
 path_header 6,x25
 path_call
 add x16,x16,#1
 cmp x16,#4
 b.ne .Lpath_client_drain
 b .Lpath_next
.Lpath_service:
 cmp x24,#6
 b.eq .Lpath_service_gate
 cmp x24,#9
 b.ne .Lpath_service_receive
.Lpath_service_gate:
 mov x14,#77
 path_submit x22,x14
 path_call
 mov x15,x1
 path_header 5,x15
 path_call
 path_header 6,x15
 path_call
.Lpath_service_receive:
 mov x16,#0
.Lpath_service_drain:
 path_header 2
 cmp x24,#6
 b.eq .Lpath_receive_start
 cmp x24,#7
 b.ne .Lpath_receive_call
.Lpath_receive_start:
 path_start
.Lpath_receive_call:
 path_call
 cmp x24,#6
 b.ne 1f
 path_end 0
1:
 cmp x24,#7
 b.ne 2f
 path_end 1
2:
 add x9,x19,#512
 ldr x25,[x9,#8]
 ldr w0,[x9,#32]
 cmp x0,x23
 b.ne .Lpath_fail
 path_header 3
 str x25,[x19,#16]
 path_call
 cmp x24,#8
 b.ne .Lpath_service_reply
 path_delay
.Lpath_service_reply:
 path_header 4
 str x25,[x19,#16]
 path_payload
 add x1,x23,#40
 bl .Lpath_call
 cbnz x0,.Lpath_fail
 cmp x24,#9
 b.ne .Lpath_next
 add x16,x16,#1
 cmp x16,#4
 b.ne .Lpath_service_drain
.Lpath_next:
 add x18,x18,#1
 cmp x18,#20
 b.ne .Lpath_round
 cbnz x20,.Lpath_shutdown_main
 cmp x24,#6
 b.eq .Lpath_shutdown_gate
 cmp x24,#9
 b.ne .Lpath_pass
.Lpath_shutdown_gate:
 path_header 8,x22
 path_call
 b .Lpath_pass
.Lpath_shutdown_main:
 path_header 8
 path_call
.Lpath_pass:
 mov x0,#1
 svc #0x4e
.Lpath_fail:
 mov x29,x0
 mov x0,x28
 svc #0x4e
 b .Lpath_fail
.Lpath_call:
 mov x0,x19
 add x2,x19,#512
 mov x3,#296
 svc #0xa0
 ret
ipc_paths_benchmark_image_end:
