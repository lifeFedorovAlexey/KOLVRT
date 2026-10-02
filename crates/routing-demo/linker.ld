USER_LOAD_ADDRESS = 0x20040000;
ENTRY(_start)
SECTIONS {
 . = USER_LOAD_ADDRESS;
 .text : { KEEP(*(.text.entry)) *(.text .text.*) }
 .rodata : { *(.rodata .rodata.*) }
 .data : { *(.data .data.*) }
 .bss : { *(.bss .bss.*) *(COMMON) }
 /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
