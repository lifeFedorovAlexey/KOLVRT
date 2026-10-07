ENTRY(_start)
PHDRS { image PT_LOAD FLAGS(5); }
SECTIONS {
  . = 0x20040000;
  .text : ALIGN(4096) { KEEP(*(.text.entry)) *(.text .text.*) *(.rodata .rodata.*) } :image
  .data : { *(.data .data.*) } :image
  .bss : { *(.bss .bss.*) *(COMMON) } :image
  ASSERT(SIZEOF(.data) == 0 && SIZEOF(.bss) == 0, "initial native ELF profile uses private stack state")
  /DISCARD/ : { *(.eh_frame .eh_frame_hdr) *(.comment) *(.note .note.*) }
}
