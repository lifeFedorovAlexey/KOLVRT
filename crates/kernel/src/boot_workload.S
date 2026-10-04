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
.equ LIFECYCLE_SPIN, 3
.equ LIFECYCLE_WAIT, 4
.equ LIFECYCLE_WAIT_TRAP, 0x55
.equ LIFECYCLE_UNSUPPORTED_CREATE, 0x60
.global lifecycle_image_start,lifecycle_image_end
lifecycle_image_start:
 mov x20,x0
 mov x21,x1
 mov x22,x2
 mov x23,x3
 mov x24,x4
 str x20,[x21]
 cmp x22,#LIFECYCLE_WAIT
 b.eq 12f
 cmp x22,#LIFECYCLE_SPIN
 b.eq 11f
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
11:
 add x25,x25,#1
 str x25,[x21,#8]
 b 11b
12:
 mov x26,#2
13:
 svc #LIFECYCLE_WAIT_TRAP
 add x25,x25,#1
 str x25,[x21,#8]
 subs x26,x26,#1
 b.ne 13b
 mov x0,x20
 svc #USER_FINISHED_TRAP
 b 13b
lifecycle_image_end:

// Actual EL0 initializes request + several-page byte buffer, then mutates its
// original request after snapshot admission. The second SVC must use saved bytes.
.global copy_image_start,copy_image_end
copy_image_start:
 mov x20,#0x2000
 lsl x20,x20,#16
 add x20,x20,#4096
 mov x0,#42
 str x0,[x20]
 mov x21,#0x2000
 lsl x21,x21,#16
 add x21,x21,#3,lsl #12
 mov x2,#0x4000
 mov w3,#0xa5
20:
 strb w3,[x21],#1
 subs x2,x2,#1
 b.ne 20b
 mov x1,x20
 svc #0x70
 mov x22,x0
 mov x0,#43
 str x0,[x20]
 mov x1,x20
 svc #0x71
 and x0,x0,x22
 svc #USER_FINISHED_TRAP
 b .
copy_image_end:

// INV-USER-IMAGE: trusted handle fixture, copied into private RX code page.
.section .rodata.handle_image,"a"
.balign 16
.global handle_image_start,handle_image_end
handle_image_start:
 mov x22,x2
 mov x19,#1
 mov x21,#0x20000000
 add x21,x21,#4096
 mov x1,x21
 svc #0x81
 and x19,x19,x0
 ldr x20,[x21,#64]
 mov x3,#1
 str x3,[x21]
 str x3,[x21,#8]
 str x20,[x21,#16]
 str x3,[x21,#24]
 mov x0,x21
 svc #0x80
 cmp x0,#0
 cset x3,eq
 and x19,x19,x3
 mov x3,#2
 str x3,[x21,#24]
 mov x0,x21
 svc #0x80
 cmp x0,#3
 cset x3,eq
 and x19,x19,x3
 mov x3,#1
 str x3,[x21,#24]
 mov x3,#-1
 str x3,[x21,#16]
 mov x0,x21
 svc #0x80
 cmp x0,#1
 cset x3,eq
 and x19,x19,x3
 str x20,[x21,#16]
 mov x3,#2
 str x3,[x21,#8]
 mov x0,x21
 svc #0x80
 cmp x0,#0
 cset x3,eq
 and x19,x19,x3
 mov x0,x21
 svc #0x80
 cmp x0,#2
 cset x3,eq
 and x19,x19,x3
 mov x3,#1
 str x3,[x21,#8]
 mov x0,x21
 svc #0x80
 cmp x0,#2
 cset x3,eq
 and x19,x19,x3
 mov x1,x21
 svc #0x82
 and x19,x19,x0
 // Old user-supplied handle must stay stale after the slot contains a new target.
 mov x3,#1
 str x3,[x21]
 str x3,[x21,#8]
 str x20,[x21,#16]
 str x3,[x21,#24]
 str xzr,[x21,#32]
 str xzr,[x21,#40]
 mov x0,x21
 svc #0x80
 cmp x0,#2
 cset x3,eq
 and x19,x19,x3
 cbz x22,1f
 // A failed suite must exit 0 rather than hide behind the expected fault.
 cbz x19,1f
 mrs x0,currentel
1:
 cbz x24,2f
 // User initiated receiver-local transfer: SEND only, with trusted
 // receiver identity supplied by this process's provisioned EL0 fixture.
 ldr x20,[x21,#64]
 mov x3,#1
 str x3,[x21]
 mov x3,#3
 str x3,[x21,#8]
 str x20,[x21,#16]
 str x23,[x21,#24]
 str x24,[x21,#32]
 mov x3,#1
 str x3,[x21,#40]
 mov x0,x21
 svc #0x80
 mov x3,#0x100
 cmp x0,x3
 cset x3,eq
 and x19,x19,x3
 // Revoke is a copied native request; future SEND admissions are denied.
 mov x3,#1
 str x3,[x21]
 mov x3,#4
 str x3,[x21,#8]
 str x20,[x21,#16]
 str xzr,[x21,#24]
 str xzr,[x21,#32]
 str xzr,[x21,#40]
 // Each malformed revoke field must reject independently before any effect.
 mov x3,#2
 str x3,[x21]
 mov x0,x21
 svc #0x80
 cmp x0,#1
 cset x3,eq
 and x19,x19,x3
 mov x0,x20
 svc #0x86
 cmp x0,#0
 cset x3,eq
 and x19,x19,x3
 mov x3,#1
 str x3,[x21]
 mov x26,#24
.Lhandle_revoke_reserved:
 mov x3,#1
 str x3,[x21,x26]
 mov x0,x21
 svc #0x80
 cmp x0,#1
 cset x3,eq
 and x19,x19,x3
 mov x0,x20
 svc #0x86
 cmp x0,#0
 cset x3,eq
 and x19,x19,x3
 str xzr,[x21,x26]
 add x26,x26,#8
 cmp x26,#48
 b.ne .Lhandle_revoke_reserved
 mov x0,x21
 svc #0x80
 cmp x0,#0
 cset x3,eq
 and x19,x19,x3
 mov x0,x20
 svc #0x86
 cmp x0,#11
 cset x3,eq
 and x19,x19,x3
 // Close the source immediately; the receiver's handle must keep the target alive.
 mov x3,#1
 str x3,[x21]
 mov x3,#2
 str x3,[x21,#8]
 str x20,[x21,#16]
 mov x3,#1
 str x3,[x21,#24]
 str xzr,[x21,#32]
 str xzr,[x21,#40]
 mov x0,x21
 svc #0x80
 cmp x0,#0
 cset x3,eq
 and x19,x19,x3
2:
 mov x0,x19
 svc #0x4e
 b .
handle_image_end:

.section .rodata.handle_foreign,"a"
.balign 16
.global handle_foreign_start,handle_foreign_end
handle_foreign_start:
 cbz x25,2f
 svc #0x55
2:
 mov x23,#0x20000000
 add x23,x23,#4096
 mov x3,#1
 str x3,[x23]
 str x3,[x23,#8]
 str x20,[x23,#16]
 str x3,[x23,#24]
 mov x0,x23
 svc #0x80
 cmp x0,x21
 cset x19,eq
 cmp x21,#2
 b.eq 1f
 mov x0,x20
 svc #0x86
 cmp x0,#0
 cset x0,eq
 and x19,x19,x0
 mov x0,x20
 svc #0x85
 cmp x0,#0
 cset x0,eq
 and x19,x19,x0
 b 2f
1:
 str x22,[x23,#16]
 mov x0,x23
 svc #0x80
 cmp x0,#0
 cset x0,eq
 and x19,x19,x0
 mov x0,x22
 svc #0x86
 // The waited receiver runs after source revocation; its alias remains live
 // for lookup/close, but a new SEND must fail with Revoked. The independent
 // cross-CPU close/lifetime scenario still requires a successful SEND.
 cbz x25,.Lhandle_alias_signal_unrevoked
 cmp x0,#11
 b .Lhandle_alias_signal_checked
.Lhandle_alias_signal_unrevoked:
 cmp x0,#0
.Lhandle_alias_signal_checked:
 cset x0,eq
 and x19,x19,x0
2:
 mov x0,x19
 svc #0x4e
 b .
handle_foreign_end:
