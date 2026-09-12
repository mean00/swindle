/*
 * RV32EC Flash Stub for CH32V003
 *
 * r10 (a0) = destination address (flash)
 * r11 (a1) = source address (ram)
 * r12 (a2) = length in bytes (must be multiple of 64)
 */

#define FLASH_CTLR 0x40022010
#define FLASH_STATR 0x4002200C
#define FLASH_ADDR 0x40022014

#define CTLR_FTPG   (1 << 16)
#define CTLR_BUFLOAD (1 << 21)
#define CTLR_STRT   (1 << 6)
#define CTLR_BUFRST (1 << 20)

#define STATR_BSY   (1 << 0)
#define STATR_EOP   (1 << 5)

    .text
    .global _start
_start:
    // a3 = FLASH_CTLR base (0x40022000)
    lui a3, 0x40022
    
    // a4 = length left
    mv a4, a2

page_loop:
    beqz a4, done

    // 5. BUFRST
    lw a5, 16(a3)      // read CTLR
    li a2, (1 << 20)   // BUFRST
    or a5, a5, a2
    sw a5, 16(a3)      // write CTLR
    
wait_bufrst:
    lw a5, 12(a3)      // read STATR
    andi a5, a5, 1     // BSY
    bnez a5, wait_bufrst

    // Clear EOP
    li a5, (1 << 5)
    sw a5, 12(a3)      // write STATR

    // 7-9. Load 64 bytes (16 words)
    li t0, 16
load_loop:
    lw a5, 0(a1)       // read word from RAM
    addi a1, a1, 4     // src += 4
    
    sw a5, 0(a0)       // write to FLASH (flash_addr)
    addi a0, a0, 4     // dst += 4

    lw a5, 16(a3)      // read CTLR
    li a2, (1 << 21)   // BUFLOAD
    or a5, a5, a2
    sw a5, 16(a3)      // write CTLR

wait_bufload:
    lw a5, 12(a3)      // read STATR
    andi a5, a5, 1
    bnez a5, wait_bufload

    addi t0, t0, -1
    bnez t0, load_loop

    // 10. Write ADDR (a0 is now past the page, subtract 64)
    addi a5, a0, -64
    sw a5, 20(a3)      // write FLASH_ADDR (0x40022014)

    // 11. set STRT
    lw a5, 16(a3)      // read CTLR
    li a2, (1 << 6)    // STRT
    or a5, a5, a2
    sw a5, 16(a3)      // write CTLR

wait_strt:
    lw a5, 12(a3)      // read STATR
    andi a5, a5, 1
    bnez a5, wait_strt

    // Clear EOP
    li a5, (1 << 5)
    sw a5, 12(a3)

    addi a4, a4, -64
    j page_loop

done:
    li a0, 0           // success
    ebreak
