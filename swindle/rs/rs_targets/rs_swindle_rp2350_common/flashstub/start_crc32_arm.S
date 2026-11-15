    .syntax unified
    .thumb
    .section .entry, "ax"
    .global _start
    .type _start, %function
_start:
    sub     sp, #8
    mov     r3, sp
    bl      rp2350_crc32
    cmp     r0, #0
    beq     .Lfail
    ldr     r1, [sp]
    add     sp, #8
    bkpt    #1
.Lfail:
    add     sp, #8
    bkpt    #0
