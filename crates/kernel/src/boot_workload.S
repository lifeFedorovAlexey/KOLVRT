.equ GPR_BYTES, 8
.equ STACK_ALIGNMENT, 16
.equ USER_TICKS_OFFSET, GPR_BYTES
.equ USER_TAG_OFFSET, (2 * GPR_BYTES)
.equ USER_FPCR_ROUND_SHIFT, 22
.equ USER_FPSR_INEXACT_SHIFT, 4
.equ USER_ROUNDING_MASK, 3
.equ USER_FINISH_SLICES, 3
.equ USER_WORKER_FINISH_SLICES, 5
.equ USER_FINISHED_TRAP, 0x4e
.equ USER_SLICES_TRAP, 0x52
.equ USER_MODE_READ, 1
.equ USER_MODE_WRITE, 2
.equ USER_MODE_EXECUTE, 3
.equ USER_MODE_PRIVILEGED, 4
.equ USER_SIMD_LOW, 0x55
.equ USER_SIMD_HIGH, 0x66
// Trusted static workload image copied into each separately owned RX page. No loader/ABI.
.section .text.user_images,"ax"
.balign 4
.global user_image_start,user_image_end
user_image_start:
 mov x20,x4
 mov x21,x0
 mov x22,x2
 mov x23,x3
 mov x24,x1
 mov x19,xzr
 movi v0.16b,#USER_SIMD_LOW
 movi v31.16b,#USER_SIMD_HIGH
 and x7,x21,#USER_ROUNDING_MASK
 lsl x7,x7,#USER_FPCR_ROUND_SHIFT
 msr fpcr,x7
 and x7,x21,#1
 lsl x7,x7,#USER_FPSR_INEXACT_SHIFT
 msr fpsr,x7
 str x20,[x24,#USER_TAG_OFFSET]
 str x20,[sp,#-STACK_ALIGNMENT]!
1:
 add x19,x19,#1
 str x19,[x24]
 // Workload observes its own native slice count; IRQ never writes fixture data.
 svc #USER_SLICES_TRAP
 mov x5,x0
 str x5,[x24,#USER_TICKS_OFFSET]
 cbnz x23,7f
 cmp x5,#USER_WORKER_FINISH_SLICES
 b.lo 1b
 b 2f
7:
 cmp x5,#USER_FINISH_SLICES
 b.lo 1b
 cbz x23,2f
 cmp x23,#USER_MODE_WRITE
 b.eq 3f
 cmp x23,#USER_MODE_EXECUTE
 b.eq 4f
 cmp x23,#USER_MODE_PRIVILEGED
 b.eq 5f
 ldr x0,[x22]
 b 6f
3:
 str x20,[x22]
 b 6f
4:
 br x22
5:
 mrs x0,sctlr_el1
6:
 mov x0,xzr
 svc #USER_FINISHED_TRAP
 b 6b
2:
 ldr x0,[sp],#STACK_ALIGNMENT
 cmp x0,x20
 cset x0,eq
 mrs x6,tpidr_el0
 cmp x6,x21
 cset x6,eq
 and x0,x0,x6
 svc #USER_FINISHED_TRAP
 b 6b
user_image_end:

// Narrow lifecycle fixture; no loader or public process ABI.
.equ LIFECYCLE_EXIT, 0
.equ LIFECYCLE_FAULT, 1
.equ LIFECYCLE_CREATE_DENIED, 2
.equ LIFECYCLE_UNSUPPORTED_CREATE, 0x60
.global lifecycle_image_start,lifecycle_image_end
lifecycle_image_start:
 mov x20,x0
 mov x21,x1
 mov x22,x2
 mov x23,x3
 mov x24,x4
 str x20,[x21]
8:
 svc #USER_SLICES_TRAP
 cmp x0,x23
 b.lo 8b
 cmp x22,#LIFECYCLE_FAULT
 b.eq 9f
 cmp x22,#LIFECYCLE_CREATE_DENIED
 b.eq 10f
 mov x0,x20
 svc #USER_FINISHED_TRAP
 b 8b
9:
 ldr x0,[x24]
 b 8b
10:
 svc #LIFECYCLE_UNSUPPORTED_CREATE
 b 8b
lifecycle_image_end:
