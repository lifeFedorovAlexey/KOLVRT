.equ PROBE_BRK_TAG, 0x4b
.equ PROBE_SIMD_LOW_PATTERN, 0x55
.equ PROBE_SIMD_HIGH_PATTERN, 0x66
// INV-PROBE: test-only exact fault PCs; handler resumes only these instructions.
.section .text.probes,"ax"
.global probe_read, probe_read_pc, probe_write, probe_write_pc, probe_break, probe_break_pc
probe_read:
probe_read_pc:
 ldr x0,[x0]
 ret
probe_write:
probe_write_pc:
 str x1,[x0]
 ret
probe_break:
probe_break_pc:
 brk #PROBE_BRK_TAG
 ret
.global probe_execute,probe_execute_resume
probe_execute:
 stp x29,x30,[sp,#-16]!
 blr x0
probe_execute_resume:
 ldp x29,x30,[sp],#16
 ret
.global probe_simd_irq
probe_simd_irq:
 // INV-PROBE: enter IRQ-masked; initialize every tested value before service.
 // A timer pending before this call must still interrupt the initialized probe.
 msr daifset,#2
 isb
 sub sp,sp,#32
 stp x29,x30,[sp]
 mrs x4,fpcr
 mrs x5,fpsr
 stp x4,x5,[sp,#16]
 msr fpcr,xzr
 msr fpsr,xzr
 mov x3,x0
 mov x2,#PROBE_SIMD_LOW_PATTERN
 dup v0.2d,x2
 mov x2,#PROBE_SIMD_HIGH_PATTERN
 dup v31.2d,x2
1:
 ldar x2,[x3]
 cbnz x2,2f
 isb // INV-TIMER: counter observation follows preceding instructions.
 mrs x2,cntpct_el0
 cmp x2,x1
 b.hs 4f
 // IRQ-masked check-to-WFI preserves a pending wake; idle yields to QEMU
 // timer callbacks without consuming the only IRQ before sleeping.
 wfi
 msr daifclr,#2
 isb
 msr daifset,#2
 isb
 b 1b
4:
 mov x0,#0
 b 3f
2:
 umov x2,v0.d[0]
 cmp x2,#PROBE_SIMD_LOW_PATTERN
 cset x0,eq
 umov x2,v31.d[1]
 cmp x2,#PROBE_SIMD_HIGH_PATTERN
 cset x2,eq
 and x0,x0,x2
 mrs x2,fpcr
 cmp x2,#0
 cset x2,eq
 and x0,x0,x2
 mrs x2,fpsr
 cmp x2,#0
 cset x2,eq
 and x0,x0,x2
3:
 ldp x4,x5,[sp,#16]
 msr fpcr,x4
 msr fpsr,x5
 ldp x29,x30,[sp]
 add sp,sp,#32
 ret
