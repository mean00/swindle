    .section .entry, "ax"
    .global _start
    .type _start, %function
_start:
    call    rp2350_flash_write
    /* rp2350_flash_write() answers 1 on success; publish the common stub exit code. */
    snez    a0, a0
    ebreak
