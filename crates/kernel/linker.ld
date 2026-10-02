KERNEL_LOAD_ADDRESS = 0x40200000;
PAGE_BYTES = 4096;
KERNEL_STACK_BYTES = 0x40000;
ENTRY(_start)
SECTIONS {
 . = KERNEL_LOAD_ADDRESS;
 __kernel_start = .;
 .text : ALIGN(PAGE_BYTES) { KEEP(*(.text.boot)) *(.text .text.*) }
 . = ALIGN(PAGE_BYTES); __text_end = .;
 .rodata : { *(.rodata .rodata.*) }
 . = ALIGN(PAGE_BYTES); __rodata_end = .;
 .data : { *(.data .data.*) }
 . = ALIGN(PAGE_BYTES); __bss_start = .;
 .bss (NOLOAD) : { *(.bss .bss.*) *(COMMON) }
 . = ALIGN(PAGE_BYTES); __bss_end = .;
 .stack (NOLOAD) : { . += KERNEL_STACK_BYTES; __stack_top = .; }
 . = ALIGN(PAGE_BYTES); __kernel_end = .;
 /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
