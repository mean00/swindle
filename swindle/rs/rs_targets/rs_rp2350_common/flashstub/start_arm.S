    .syntax unified
    .thumb
    .section .entry, "ax"
    .global _start
    .type _start, %function
_start:
    bl      rp2350_flash_write
    cmp     r0, #0
    beq     .Lfail
    bkpt    #1
.Lfail:
    bkpt    #0
