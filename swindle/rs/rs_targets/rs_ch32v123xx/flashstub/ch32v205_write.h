#pragma once
#include <stddef.h>
#include <stdint.h>

/*
 * CH32V205 flash programming stub -- the driver's only C header.
 *
 * The CH32V205 is a *special case* in this crate: same memory map, same FPEC base and the
 * same unlock keys as the CH32V2xx/V3xx parts, but a different programming core.
 *
 * `ch32v3.md` (CH32F/V20x_V30x_V31x Reference Manual, chapter 32 "FLASH") is the document
 * the rest of this crate follows, and its device list is CH32F20x / CH32V20x (CH32V203,
 * CH32V208) / CH32V30x / CH32V31x -- the CH32V205 does not appear in it (only in the USB
 * OTG figures). WCH's CH32V205 EVT (`ch32v205_flash.c`, V1.0.3) drives the buffer-based
 * controller of the CH32V00x family instead, which is what this header describes:
 *
 *   chapter 32 (CH32V203/V208/V30x)     CH32V205
 *   FTPG = bit 16, fast page program    PAGE_PG  = bit 16
 *   FTER = bit 17, 256 B page erase     PAGE_ER  = bit 17
 *   BER32 = bit 18, 32 KB erase         BUF_LOAD = bit 18
 *   BER64 = bit 19 (RM V2.2 only)       BUF_RST  = bit 19
 *   PGSTRT = bit 21, start fast pgm     BER64    = bit 23
 *   erase leaves 0xE339E339             no plain PG (bit 0) mode at all
 *
 * Programming is therefore a 64-word page buffer sequence, not "write the flash window and
 * set PGSTRT". The debugger unlocks FPEC through FLASH_KEYR and the fast mode through
 * FLASH_MODEKEYR before starting the stub, exactly as for the chapter 32 parts.
 */

/* FPEC register file, same layout as `src/registers.rs::FlashRegisters` */
#define CH32V205_FLASH_CONTROLLER_ADDRESS 0x40022000U

typedef struct
{
    uint32_t ACTLR;       /* 0x00 control */
    uint32_t KEYR;        /* 0x04 FPEC key */
    uint32_t OBKEYR;      /* 0x08 option byte key */
    uint32_t STATR;       /* 0x0c status */
    uint32_t CTLR;        /* 0x10 configuration */
    uint32_t ADDR;        /* 0x14 address */
    uint32_t RESERVED_18; /* 0x18 */
    uint32_t OBR;         /* 0x1c option byte */
    uint32_t WPR;         /* 0x20 write protection */
    uint32_t MODEKEYR;    /* 0x24 extended (fast mode) key */
} ch32v205_flash_s;

/* Datasheet tripwire: fails the build if the struct is edited so that the extended key
 * register no longer sits at its documented offset. */
_Static_assert(offsetof(ch32v205_flash_s, MODEKEYR) == 0x24, "FPEC register file layout");

/* FLASH_KEYR / FLASH_MODEKEYR unlock keys (RM 27.4.2 / 27.4.4), written in order */
#define CH32V205_FLASH_KEY1 0x45670123U
#define CH32V205_FLASH_KEY2 0xcdef89abU

/* FLASH_STATR: BSY, then the two RW1C flags */
#define CH32V205_FMC_STAT_BUSY (1U << 0U)
#define CH32V205_FMC_STAT_WRPRTERR (1U << 4U)
#define CH32V205_FMC_STAT_EOP (1U << 5U)
/* The flags are RW1C: writing this mask clears them and touches nothing else */
#define CH32V205_FMC_STAT_CLEAR (CH32V205_FMC_STAT_EOP | CH32V205_FMC_STAT_WRPRTERR)

/* FLASH_CTLR: only the bits the stubs drive (RM V1.2, 27.3.5) */
#define CH32V205_FMC_CTL_PER (1U << 1U)      /* sector erase (2 KiB) */
#define CH32V205_FMC_CTL_STRT (1U << 6U)     /* start, cleared by hardware */
#define CH32V205_FMC_CTL_LOCK (1U << 7U)     /* FPEC locked */
#define CH32V205_FMC_CTL_FLOCK (1U << 15U)   /* fast mode locked */
#define CH32V205_FMC_CTL_FTPG (1U << 16U)    /* 256 B page program mode */
#define CH32V205_FMC_CTL_FTER (1U << 17U)    /* 256 B page erase mode */
#define CH32V205_FMC_CTL_BUFLOAD (1U << 18U) /* cache the written word in the buffer */
#define CH32V205_FMC_CTL_BUFRST (1U << 19U)  /* clear the buffer */
#define CH32V205_FMC_CTL_BER64 (1U << 23U)   /* 64 KiB block erase */

/* Erase granularity per mode: PER = 2 KiB sector, FTER = 256 B page */
#define CH32V205_SECTOR_BYTES 2048U
#define CH32V205_BLOCK64_BYTES 65536U

/* Programming geometry (RM V1.2, Table 27-1: page = 256 bytes) */
#define CH32V205_FLASH_PAGE_BYTES 256U
#define CH32V205_FLASH_WORD_BYTES 4U
