    .section .entry, "ax"
    .global _start
    .type _start, %function
_start:
    addi    sp, sp, -16
    addi    a3, sp, 8
    call    rp2350_crc32
    beqz    a0, .Lfail
    lw      a2, 8(sp)
    addi    sp, sp, 16
    li      a0, 1
    ebreak
.Lfail:
    addi    sp, sp, 16
    li      a0, 0
    ebreak
