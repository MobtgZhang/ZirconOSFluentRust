# Physical load at 128 MiB (ZBM10 + objcopy flat NT10KRNL.BIN).
# OVMF/QEMU often marks low RAM (e.g. below ~64–128 MiB) busy for AllocateAddress; 128 MiB is a stable base.
# Include .got and .data.rel.ro here; otherwise lld orphans them far above .rodata (~360 MiB VMA hole).
ENTRY(_start)

SECTIONS
{
    . = 0x8000000;

    .text : ALIGN(4096) {
        *(.text.boot .text.boot.*)
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    .data.rel.ro : ALIGN(4096) {
        *(.data.rel.ro .data.rel.ro.*)
    }

    .got : ALIGN(4096) {
        *(.got .got.*)
        *(.got.plt .got.plt.*)
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
