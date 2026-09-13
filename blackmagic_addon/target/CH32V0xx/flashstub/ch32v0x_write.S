/*
 * RV32EC Flash Stub for CH32V003
 *
 * a0 = destination address (flash)
 * a1 = source address (ram)
 * a2 = length in bytes (must be multiple of 64)
 * a3 = base FLASH_CTLR value (ctlr_saved | CTLR_FTPG)
 */

    .text
    .global _start
_start:
    // t1 = FLASH base (0x40022000)
    lui t1, 0x40022
    
    // t2 = length left
    mv t2, a2

page_loop:
    beqz t2, done

    // 5. BUFRST
    li a2, (1 << 20)   // BUFRST
    or a5, a3, a2      // a3 contains the clean CTLR base
    sw a5, 16(t1)      // write CTLR
    
wait_bufrst:
    lw a5, 12(t1)      // read STATR
    andi a2, a5, 1     // BSY
    bnez a2, wait_bufrst

    // Clear STATR (write back read value to clear EOP and errors)
    sw a5, 12(t1)

    // 7-9. Load 64 bytes (16 words)
    li t0, 16
load_loop:
    lw a5, 0(a1)       // read word from RAM
    addi a1, a1, 4     // src += 4
    sw a5, 0(a0)       // write to FLASH (flash_addr)
    addi a0, a0, 4     // dst += 4

    li a2, (1 << 21)   // BUFLOAD
    or a5, a3, a2      // clean CTLR + BUFLOAD
    sw a5, 16(t1)      // write CTLR

wait_bufload:
    lw a5, 12(t1)      // read STATR
    andi a2, a5, 1
    bnez a2, wait_bufload

    addi t0, t0, -1
    bnez t0, load_loop

    // 10. Write ADDR (a0 is now past the page, subtract 64)
    addi a5, a0, -64
    sw a5, 20(t1)      // write FLASH_ADDR (0x40022014)

    // 11. set STRT
    li a2, (1 << 6)               // STRT only
    or a5, a3, a2                 // clean CTLR + STRT
    sw a5, 16(t1)                 // write CTLR

wait_strt:
    lw a5, 12(t1)      // read STATR
    andi a2, a5, 1
    bnez a2, wait_strt

    // Clear STATR
    sw a5, 12(t1)

    addi t2, t2, -64
    j page_loop

done:
    li a0, 0           // success
    ebreak
