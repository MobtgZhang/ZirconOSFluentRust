/* Minimal linker script for x86_64 bare-metal kernel (stub). Expand for real image layout. */
ENTRY(_start)

SECTIONS
{
    . = 0xFFFFFFFF80000000;

    .text : ALIGN(4096) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4096) {
        *(.data .data.*)
    }

    .bss : ALIGN(4096) {
        *(COMMON)
        *(.bss .bss.*)
    }

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note .note.*)
    }
}
