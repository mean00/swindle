#pragma once
#include <stdint.h>

/*
 * IDCODE register (read through the DM system-bus access at 0x1ffff7c4)
 * [31:16] - REVID
 * [15:0]  - DEVID
 */
#define CH32V003X_IDCODE 0x1ffff7c4U
#define CH32V0X_IDCODE_MASK 0x0ffffff0fU

/* Electronic Signature (ESIG) registers */
#define CH32V0X_ESIG_FLASH_CAP 0x1ffff7e0U /* Flash capacity register, 16 bits, KiB units */
#define CH32V0X_ESIG_UID1 0x1ffff7e8U      /* Unique ID register, bits 0:31 */
#define CH32V0X_ESIG_UID2 0x1ffff7ecU      /* Unique ID register, bits 32:63 */
#define CH32V0X_ESIG_UID3 0x1ffff7f0U      /* Unique ID register, bits 64:95 */

#define RAM_SIZE 2 // default RAM size, 2kB on the CH32V003 family
#define RAM_ADDRESS 0x20000000UL
/*
 * The 0x0000 0000 region is a boot alias ("Aliased to Flash or system memory",
 * chapter 1.2/option byte MODE) and is what a CH32V003 ELF is linked at, so it
 * is the window exposed to GDB. The FPEC itself only decodes the physical
 * CodeFlash window 0x0800 0000-0x0800 3FFF (table 16-1), so every address
 * handed to FLASH_ADDR or written for the fast-program buffer load has to be
 * translated by FLASH_OFFSET first.
 */
#define FLASH_ADDRESS 0x00000000UL
#define FLASH_OFFSET 0x08000000UL

/*
 * Flash programming (CH32V003 Reference Manual V1.3, chapter 16 "FLASH").
 *
 * The FPEC (flash program/erase controller) registers are memory mapped and are
 * driven from the debugger through the debug module system bus, the very same
 * access path used for the ESIG/IDCODE reads above. Two lock layers gate
 * programming: FPEC/FLASH_CTLR (unlocked via FLASH_KEYR, 16.4.2) and the fast
 * programming mode (unlocked via FLASH_MODEKEYR, 16.4.5). Erase and programming
 * are done in "fast page" mode (16.4.6/16.4.7), which works in 64 byte pages --
 * the same granularity exposed to the GDB flash layer as the flash block size.
 */

/* Table 16-2: FPEC register file */
#define CH32V0X_FLASH_CONTROLLER_ADDRESS 0x40022000U

/* 16.3.2/16.4.2: FPEC unlock keys, written in order to FLASH_KEYR */
#define CH32V0X_FLASH_KEY1 0x45670123U
#define CH32V0X_FLASH_KEY2 0xcdef89abU

/* 16.3.4: FLASH_STATR flags, EOP and WRPRTERR are cleared by writing 1 to them */
#define CH32V0X_FMC_STAT_BUSY (1U << 0U)     /* Flash operation in progress */
#define CH32V0X_FMC_STAT_WRPRTERR (1U << 4U) /* Write protection error */
#define CH32V0X_FMC_STAT_EOP (1U << 5U)      /* End of operation */

/* 16.3.5: FLASH_CTLR control bits */
#define CH32V0X_FMC_CTL_PG (1U << 0U)       /* Standard (half word) programming */
#define CH32V0X_FMC_CTL_PER (1U << 1U)      /* Standard page (1 KiB) erase */
#define CH32V0X_FMC_CTL_MER (1U << 2U)      /* Whole chip erase */
#define CH32V0X_FMC_CTL_STRT (1U << 6U)     /* Start erase/program, cleared by hardware */
#define CH32V0X_FMC_CTL_LOCK (1U << 7U)     /* FPEC and FLASH_CTLR locked */
#define CH32V0X_FMC_CTL_FLOCK (1U << 15U)   /* Fast programming/erase mode locked */
#define CH32V0X_FMC_CTL_FTPG (1U << 16U)    /* Fast page programming (64 B) */
#define CH32V0X_FMC_CTL_FTER (1U << 17U)    /* Fast page erase (64 B), "PAGE_ER" in 16.4.7 */
#define CH32V0X_FMC_CTL_BUFLOAD (1U << 18U) /* Cache the written word into the 64 B buffer */
#define CH32V0X_FMC_CTL_BUFRST (1U << 19U)  /* Reset (clear) the 64 B buffer */

/* 16.4.6/16.4.7: fast page granularity and the 32 bit buffer load width */
#define CH32V0X_FLASH_PAGE_BYTES 64U
#define CH32V0X_FLASH_WORD_BYTES 4U
#define CH32V0X_FLASH_PAGE_WORDS (CH32V0X_FLASH_PAGE_BYTES / CH32V0X_FLASH_WORD_BYTES)

/* FPEC operation timeout, generous compared to the ~1 ms page program/erase times */
#define CH32V0X_FLASH_TIMEOUT_MS 50U

/* Table 16-2 register file layout, byte offsets in the comments */
typedef struct
{
    uint32_t ACTLR;         /* 0x00 control */
    uint32_t KEYR;          /* 0x04 FPEC key */
    uint32_t OBKEYR;        /* 0x08 option byte key */
    uint32_t STATR;         /* 0x0c status */
    uint32_t CTLR;          /* 0x10 configuration */
    uint32_t ADDR;          /* 0x14 address */
    uint32_t RESERVED_18;   /* 0x18 */
    uint32_t OBR;           /* 0x1c option byte */
    uint32_t WPR;           /* 0x20 write protection */
    uint32_t MODEKEYR;      /* 0x24 extended (fast mode) key */
    uint32_t BOOT_MODEKEYR; /* 0x28 BOOT area key */
} ch32v0x_flash_s;
// EOF
