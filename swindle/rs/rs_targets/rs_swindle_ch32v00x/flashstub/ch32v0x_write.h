#pragma once
#include <stddef.h>
#include <stdint.h>

/*
 * CH32V00x in-SRAM flash programming stub -- the driver's only C header.
 *
 * Everything the stub needs from the datasheet lives here: the FPEC register file
 * as a struct (register addresses then come from `offsetof`, as in the Rust twin
 * `src/registers.rs`), the register bits and the programming geometry. The device
 * identity / memory layout constants are not here: only the Rust driver needs
 * them (`src/registers.rs`), and it never shares this file.
 *
 * Flash programming (CH32V003 Reference Manual V1.3, chapter 16 "FLASH"):
 * the FPEC (flash program/erase controller) is unlocked from the debugger before
 * the stub is started -- FPEC/FLASH_CTLR via FLASH_KEYR (16.4.2) and the fast
 * programming mode via FLASH_MODEKEYR (16.4.5). The stub therefore only runs the
 * fast page sequence (16.4.6/16.4.7) over the chunk it was handed.
 */

/* Table 16-2: FPEC register file */
#define CH32V0X_FLASH_CONTROLLER_ADDRESS 0x40022000U

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

/* Datasheet tripwire (Table 16-2): fails the build if the struct is edited so
 * that the last register no longer sits at its documented offset. */
_Static_assert(offsetof(ch32v0x_flash_s, BOOT_MODEKEYR) == 0x28, "FPEC register file layout");

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

/* 16.4.6/16.4.7: the 32 bit buffer load width. The page size is a stub argument
 * (64 B on the CH32V003, 256 B on the CH32V006), never a constant. */
#define CH32V0X_FLASH_WORD_BYTES 4U
