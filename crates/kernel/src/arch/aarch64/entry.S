.equ GPR_BYTES, 8
.equ SIMD_BYTES, 16
.equ STACK_ALIGNMENT, 16
.equ VECTOR_ALIGNMENT, 2048
.equ VECTOR_SLOT_BYTES, 128
.equ CONTEXT_GPR_BASE, 0
.equ CONTEXT_SIMD_BASE, (32 * GPR_BYTES)
.equ CONTEXT_FPCR, (CONTEXT_SIMD_BASE + 32 * SIMD_BYTES)
.equ CONTEXT_FPSR, (CONTEXT_FPCR + GPR_BYTES)
.equ CONTEXT_BYTES, (CONTEXT_FPSR + GPR_BYTES)
.equ USER_ELR, CONTEXT_BYTES
.equ USER_SPSR, (USER_ELR + GPR_BYTES)
.equ USER_SP, (USER_SPSR + GPR_BYTES)
.equ USER_TPIDR, (USER_SP + GPR_BYTES)
.equ USER_META_BYTES, (4 * GPR_BYTES)
.equ USER_CONTEXT_BYTES, (CONTEXT_BYTES + USER_META_BYTES)
.equ USER_CONTEXT_PAIR_BYTES, (2 * GPR_BYTES)

.equ USER_TRAP_IRQ, 1
.equ PROBE_BRK_TAG, 0x4b
.equ PROBE_SIMD_LOW_PATTERN, 0x55
.equ PROBE_SIMD_HIGH_PATTERN, 0x66
.equ DAIF_MASK_ALL, 15
.equ MPIDR_AFF0_MASK, 0xff
.equ CURRENT_EL2, 8
.equ CURRENT_EL1, 4
.equ HCR_RW_AARCH64, (1 << 31)
.equ CNTHCTL_EL1_COUNTER_TIMER_ACCESS, 3
.equ ICC_SRE_EL2_SRE_ENABLE, 9
.equ SPSR_MASKED_EL1H, 0x3c5
.equ SPSEL_SP_EL1, 1
.equ CPACR_FPEN_FULL_ACCESS, (3 << 20)
// INV-ENTRY: QEMU ELF entry, MMU off, nonsecure EL1/EL2, primary affinity zero.
.section .text.boot,"ax"
.global _start
_start:
 msr daifset, #DAIF_MASK_ALL
 mrs x1, mpidr_el1
 and x1, x1, #MPIDR_AFF0_MASK
 cbnz x1, park
 mrs x1, CurrentEL
 cmp x1, #CURRENT_EL2
 b.ne 1f
 mov x1, #HCR_RW_AARCH64
 msr hcr_el2, x1
 mov x1, #CNTHCTL_EL1_COUNTER_TIMER_ACCESS
 msr cnthctl_el2, x1
 msr cntvoff_el2, xzr
 mov x1, #ICC_SRE_EL2_SRE_ENABLE
 msr S3_4_C12_C9_5, x1
 adr x1, 1f
 msr elr_el2, x1
 mov x1, #SPSR_MASKED_EL1H
 msr spsr_el2, x1
 eret
1:
 mrs x1, CurrentEL
 cmp x1, #CURRENT_EL1
 b.ne park
 msr spsel, #SPSEL_SP_EL1
 ldr x1, =__stack_top
 mov sp, x1
 ldr x1, =__bss_start
 ldr x2, =__bss_end
2:
 cmp x1, x2
 b.hs 3f
 str xzr, [x1], #GPR_BYTES
 b 2b
3:
 mov x1, #CPACR_FPEN_FULL_ACCESS
 msr cpacr_el1, x1
 isb
 bl kernel_main
park:
 wfe
 b park

// INV-SECONDARY: PSCI context in x0 is the cleaned native table root; no BSS initialization.
// Dedicated permanent stack; MMU and coherent publication established before shared Rust access.
.global secondary_entry
secondary_entry:
 msr daifset, #DAIF_MASK_ALL
 mov x19, x0
 mrs x1, CurrentEL
 cmp x1, #CURRENT_EL2
 b.ne 4f
 mov x1, #HCR_RW_AARCH64
 msr hcr_el2, x1
 mov x1, #CNTHCTL_EL1_COUNTER_TIMER_ACCESS
 msr cnthctl_el2, x1
 msr cntvoff_el2, xzr
 mov x1, #ICC_SRE_EL2_SRE_ENABLE
 msr S3_4_C12_C9_5, x1
 adr x1, 4f
 msr elr_el2, x1
 mov x1, #SPSR_MASKED_EL1H
 msr spsr_el2, x1
 eret
4:
 mrs x1, CurrentEL
 cmp x1, #CURRENT_EL1
 b.ne park
 msr spsel, #SPSEL_SP_EL1
 ldr x1, =__secondary_stack_top
 mov sp, x1
 mov x1, #CPACR_FPEN_FULL_ACCESS
 msr cpacr_el1, x1
 isb
 mov x0, x19
 bl secondary_main
 b park

// INV-VECTOR: SP_EL1 aligned 16; preserve GPR, SIMD, FPCR/FPSR; pinned target enables NEON.
.balign VECTOR_ALIGNMENT
.global vectors
vectors:
.macro slot target
 b \target
 .balign VECTOR_SLOT_BYTES
.endm
slot fatal_vector
slot fatal_vector
slot fatal_vector
slot fatal_vector
slot sync_vector
slot irq_vector
slot fatal_vector
slot fatal_vector
slot user_sync_vector
slot user_irq_vector
slot fatal_vector
slot fatal_vector
slot fatal_vector
slot fatal_vector
slot fatal_vector
slot fatal_vector
.macro save
 sub sp, sp, #CONTEXT_BYTES
 stp x0,x1,[sp,#(CONTEXT_GPR_BASE + 0 * GPR_BYTES)]
 stp x2,x3,[sp,#(CONTEXT_GPR_BASE + 2 * GPR_BYTES)]
 stp x4,x5,[sp,#(CONTEXT_GPR_BASE + 4 * GPR_BYTES)]
 stp x6,x7,[sp,#(CONTEXT_GPR_BASE + 6 * GPR_BYTES)]
 stp x8,x9,[sp,#(CONTEXT_GPR_BASE + 8 * GPR_BYTES)]
 stp x10,x11,[sp,#(CONTEXT_GPR_BASE + 10 * GPR_BYTES)]
 stp x12,x13,[sp,#(CONTEXT_GPR_BASE + 12 * GPR_BYTES)]
 stp x14,x15,[sp,#(CONTEXT_GPR_BASE + 14 * GPR_BYTES)]
 stp x16,x17,[sp,#(CONTEXT_GPR_BASE + 16 * GPR_BYTES)]
 stp x18,x19,[sp,#(CONTEXT_GPR_BASE + 18 * GPR_BYTES)]
 stp x20,x21,[sp,#(CONTEXT_GPR_BASE + 20 * GPR_BYTES)]
 stp x22,x23,[sp,#(CONTEXT_GPR_BASE + 22 * GPR_BYTES)]
 stp x24,x25,[sp,#(CONTEXT_GPR_BASE + 24 * GPR_BYTES)]
 stp x26,x27,[sp,#(CONTEXT_GPR_BASE + 26 * GPR_BYTES)]
 stp x28,x29,[sp,#(CONTEXT_GPR_BASE + 28 * GPR_BYTES)]
 str x30,[sp,#(CONTEXT_GPR_BASE + 30 * GPR_BYTES)]
 str xzr,[sp,#(CONTEXT_GPR_BASE + 31 * GPR_BYTES)]
 stp q0,q1,[sp,#(CONTEXT_SIMD_BASE + 0 * SIMD_BYTES)]
 stp q2,q3,[sp,#(CONTEXT_SIMD_BASE + 2 * SIMD_BYTES)]
 stp q4,q5,[sp,#(CONTEXT_SIMD_BASE + 4 * SIMD_BYTES)]
 stp q6,q7,[sp,#(CONTEXT_SIMD_BASE + 6 * SIMD_BYTES)]
 stp q8,q9,[sp,#(CONTEXT_SIMD_BASE + 8 * SIMD_BYTES)]
 stp q10,q11,[sp,#(CONTEXT_SIMD_BASE + 10 * SIMD_BYTES)]
 stp q12,q13,[sp,#(CONTEXT_SIMD_BASE + 12 * SIMD_BYTES)]
 stp q14,q15,[sp,#(CONTEXT_SIMD_BASE + 14 * SIMD_BYTES)]
 stp q16,q17,[sp,#(CONTEXT_SIMD_BASE + 16 * SIMD_BYTES)]
 stp q18,q19,[sp,#(CONTEXT_SIMD_BASE + 18 * SIMD_BYTES)]
 stp q20,q21,[sp,#(CONTEXT_SIMD_BASE + 20 * SIMD_BYTES)]
 stp q22,q23,[sp,#(CONTEXT_SIMD_BASE + 22 * SIMD_BYTES)]
 stp q24,q25,[sp,#(CONTEXT_SIMD_BASE + 24 * SIMD_BYTES)]
 stp q26,q27,[sp,#(CONTEXT_SIMD_BASE + 26 * SIMD_BYTES)]
 stp q28,q29,[sp,#(CONTEXT_SIMD_BASE + 28 * SIMD_BYTES)]
 stp q30,q31,[sp,#(CONTEXT_SIMD_BASE + 30 * SIMD_BYTES)]
 mrs x9,fpcr
 mrs x10,fpsr
 str x9,[sp,#CONTEXT_FPCR]
 str x10,[sp,#CONTEXT_FPSR]
.endm
.macro restore extra=0
 ldp q0,q1,[sp,#(CONTEXT_SIMD_BASE + 0 * SIMD_BYTES)]
 ldp q2,q3,[sp,#(CONTEXT_SIMD_BASE + 2 * SIMD_BYTES)]
 ldp q4,q5,[sp,#(CONTEXT_SIMD_BASE + 4 * SIMD_BYTES)]
 ldp q6,q7,[sp,#(CONTEXT_SIMD_BASE + 6 * SIMD_BYTES)]
 ldp q8,q9,[sp,#(CONTEXT_SIMD_BASE + 8 * SIMD_BYTES)]
 ldp q10,q11,[sp,#(CONTEXT_SIMD_BASE + 10 * SIMD_BYTES)]
 ldp q12,q13,[sp,#(CONTEXT_SIMD_BASE + 12 * SIMD_BYTES)]
 ldp q14,q15,[sp,#(CONTEXT_SIMD_BASE + 14 * SIMD_BYTES)]
 ldp q16,q17,[sp,#(CONTEXT_SIMD_BASE + 16 * SIMD_BYTES)]
 ldp q18,q19,[sp,#(CONTEXT_SIMD_BASE + 18 * SIMD_BYTES)]
 ldp q20,q21,[sp,#(CONTEXT_SIMD_BASE + 20 * SIMD_BYTES)]
 ldp q22,q23,[sp,#(CONTEXT_SIMD_BASE + 22 * SIMD_BYTES)]
 ldp q24,q25,[sp,#(CONTEXT_SIMD_BASE + 24 * SIMD_BYTES)]
 ldp q26,q27,[sp,#(CONTEXT_SIMD_BASE + 26 * SIMD_BYTES)]
 ldp q28,q29,[sp,#(CONTEXT_SIMD_BASE + 28 * SIMD_BYTES)]
 ldp q30,q31,[sp,#(CONTEXT_SIMD_BASE + 30 * SIMD_BYTES)]
 ldr x9,[sp,#CONTEXT_FPCR]
 ldr x10,[sp,#CONTEXT_FPSR]
 msr fpcr,x9
 msr fpsr,x10
 ldp x0,x1,[sp,#(CONTEXT_GPR_BASE + 0 * GPR_BYTES)]
 ldp x2,x3,[sp,#(CONTEXT_GPR_BASE + 2 * GPR_BYTES)]
 ldp x4,x5,[sp,#(CONTEXT_GPR_BASE + 4 * GPR_BYTES)]
 ldp x6,x7,[sp,#(CONTEXT_GPR_BASE + 6 * GPR_BYTES)]
 ldp x8,x9,[sp,#(CONTEXT_GPR_BASE + 8 * GPR_BYTES)]
 ldp x10,x11,[sp,#(CONTEXT_GPR_BASE + 10 * GPR_BYTES)]
 ldp x12,x13,[sp,#(CONTEXT_GPR_BASE + 12 * GPR_BYTES)]
 ldp x14,x15,[sp,#(CONTEXT_GPR_BASE + 14 * GPR_BYTES)]
 ldp x16,x17,[sp,#(CONTEXT_GPR_BASE + 16 * GPR_BYTES)]
 ldp x18,x19,[sp,#(CONTEXT_GPR_BASE + 18 * GPR_BYTES)]
 ldp x20,x21,[sp,#(CONTEXT_GPR_BASE + 20 * GPR_BYTES)]
 ldp x22,x23,[sp,#(CONTEXT_GPR_BASE + 22 * GPR_BYTES)]
 ldp x24,x25,[sp,#(CONTEXT_GPR_BASE + 24 * GPR_BYTES)]
 ldp x26,x27,[sp,#(CONTEXT_GPR_BASE + 26 * GPR_BYTES)]
 ldp x28,x29,[sp,#(CONTEXT_GPR_BASE + 28 * GPR_BYTES)]
 ldr x30,[sp,#(CONTEXT_GPR_BASE + 30 * GPR_BYTES)]
 add sp,sp,#(CONTEXT_BYTES + \extra)
 eret
.endm
sync_vector:
 save
 mrs x0,esr_el1
 mrs x1,far_el1
 mrs x2,elr_el1
 bl synchronous
 msr elr_el1,x0
 restore
irq_vector:
 save
 bl interrupt_entry
 restore
fatal_vector:
 save
 mrs x0,esr_el1
 mrs x1,far_el1
 mrs x2,elr_el1
 bl fatal_exception

// INV-USER-CONTEXT: full GPR/SIMD/FP and exception state; EL1 stack never uses user SP.
.macro save_user
 sub sp,sp,#USER_META_BYTES
 save
 mrs x9,elr_el1
 mrs x10,spsr_el1
 str x9,[sp,#USER_ELR]
 str x10,[sp,#USER_SPSR]
 mrs x9,sp_el0
 mrs x10,tpidr_el0
 str x9,[sp,#USER_SP]
 str x10,[sp,#USER_TPIDR]
.endm
.macro return_user
 ldr x9,[sp,#USER_ELR]
 ldr x10,[sp,#USER_SPSR]
 msr elr_el1,x9
 msr spsr_el1,x10
 ldr x9,[sp,#USER_SP]
 ldr x10,[sp,#USER_TPIDR]
 msr sp_el0,x9
 msr tpidr_el0,x10
 restore USER_META_BYTES
.endm
user_sync_vector:
 save_user
 mov x0,sp
 mov x1,xzr
 mrs x2,cntpct_el0
 bl user_trap
 cbnz x0,user_kernel_return
 return_user
user_irq_vector:
 save_user
 bl interrupt_entry
 mov x2,x0
 mov x0,sp
 mov x1,#USER_TRAP_IRQ
 bl user_trap
 cbnz x0,user_kernel_return
 return_user
.global run_user
run_user:
 save_user
 mov x9,sp
 stlr x9,[x1]
 sub sp,sp,#USER_CONTEXT_BYTES
 mov x9,sp
 mov x12,#(USER_CONTEXT_BYTES / USER_CONTEXT_PAIR_BYTES)
1:
 ldp x10,x11,[x0],#USER_CONTEXT_PAIR_BYTES
 stp x10,x11,[x9],#USER_CONTEXT_PAIR_BYTES
 subs x12,x12,#1
 b.ne 1b
 return_user
user_kernel_return:
 mov sp,x0
 ldr x9,[sp,#USER_SP]
 ldr x10,[sp,#USER_TPIDR]
 msr sp_el0,x9
 msr tpidr_el0,x10
 adr x9,user_kernel_resumed
 msr elr_el1,x9
 mov x9,#SPSR_MASKED_EL1H
 msr spsr_el1,x9
 restore USER_META_BYTES
user_kernel_resumed:
 ret

// INV-USER-COPY: kernel accesses remain fatal; exact user instructions recover.
// x4 counts completed stores only. No Rust user references, masked IRQ, bounded
// disjoint buffers and immutable retained root enforced by the Rust wrapper.
.global user_copy_in,user_copy_out,user_copy_load,user_copy_store,user_copy_resume
user_copy_in:
 mov x4,xzr
1:
 cmp x4,x2
 b.eq user_copy_resume
user_copy_load:
 ldtrb w3,[x1]
 strb w3,[x0]
 add x0,x0,#1
 add x1,x1,#1
 add x4,x4,#1
 b 1b
user_copy_out:
 mov x4,xzr
2:
 cmp x4,x2
 b.eq user_copy_resume
 ldrb w3,[x1]
user_copy_store:
 sttrb w3,[x0]
 add x0,x0,#1
 add x1,x1,#1
 add x4,x4,#1
 b 2b
user_copy_resume:
 mov x0,x4
 ret

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
 mrs x2,cntpct_el0
 cmp x2,x1
 b.lo 1b
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
