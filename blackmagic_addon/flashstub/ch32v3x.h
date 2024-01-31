/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2023 1BitSquared <info@1bitsquared.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "stdbool.h"
#include "stddef.h"
#include "stdint.h"

typedef struct
{
    uint32_t WS;       // 0
    uint32_t KEYR;     // 4 aka fpec
    uint32_t OBKEYR;   // 8
    uint32_t STATR;    // C
    uint32_t CTLR;     // 10
    uint32_t ADDR;     // 14
    uint32_t filler;   // 18
    uint32_t OBR;      // 1C
    uint32_t WPR;      // 20
    uint32_t MODEKEYR; // 24
} ch32__s;

#define CH32V3XX__CONTROLLER_ADDRESS 0x40022000
#define CH32V3XX_UID1 0x1ffff7e8      // Low bits of UUID
#define CH32V3XX_FMC_CTL_PG (1 << 0)  // program command
#define CH32V3XX_FMC_CTL_PER (1 << 1) // page erase command
#define CH32V3XX_FMC_CTL_START (1 << 6)
#define CH32V3XX_FMC_CTL_LK (1 << 7)
#define CH32V3XX_FMC_CTL_CH32_FASTUNLOCK (1 << 15)
#define CH32V3XX_FMC_CTL_CH32_FASTPROGRAM (1 << 16)
#define CH32V3XX_FMC_CTL_CH32_FASTERASE (1 << 17)
#define CH32V3XX_FMC_CTL_CH32_FASTSTART (1 << 21)

#define CH32V3XX_FMC_STAT_BUSY (1 << 0)
#define CH32V3XX_FMC_STAT_WR_BUSY (1 << 1)
#define CH32V3XX_FMC_STAT_WP_ERR (1 << 4)  // erase / program erro
#define CH32V3XX_FMC_STAT_WP_ENDF (1 << 5) // end of operation
#define CH32V3XX_FMC_STAT_PG_ERR (1 << 3)  // program error
#define CH32V3XX_FMC_STAT_WP_ERR (1 << 4)  // erase / program erro

#define CH32V3XX_KEY1 0x45670123UL
#define CH32V3XX_KEY2 0xcdef89abUL

/* RISC-V targets CH32Vx */
#define CH32VX_CHIPID 0x1ffff704U
#define CH32VX_CHIPID_FAMILY_OFFSET 20U
#define CH32VX_CHIPID_FAMILY_MASK (0xfffU << CH32VX_CHIPID_FAMILY_OFFSET)

#define READ__REG(target, reg) (*(volatile uint32_t *)(CH32V3XX__CONTROLLER_ADDRESS + offsetof(ch32__s, reg)))
#define WRITE__REG(target, reg, value)                                                                                 \
    {                                                                                                                  \
        (*(volatile uint32_t *)(CH32V3XX__CONTROLLER_ADDRESS + offsetof(ch32__s, reg))) = value;                       \
    }

#define target 999
#define INLINE inline __attribute__((always_inline))

#define target_mem_write32(xx, adr, value)                                                                             \
    {                                                                                                                  \
        *(volatile uint32_t *)adr = value;                                                                             \
    }
/*
 */
#define ch32v3x_fast_unlock()                                                                                          \
    {                                                                                                                  \
        uint32_t ctl = READ__REG(target, CTLR);                                                                        \
                                                                                                                       \
        WRITE__REG(target, KEYR, CH32V3XX_KEY1);                                                                       \
        WRITE__REG(target, KEYR, CH32V3XX_KEY2);                                                                       \
                                                                                                                       \
        WRITE__REG(target, MODEKEYR, CH32V3XX_KEY1);                                                                   \
        WRITE__REG(target, MODEKEYR, CH32V3XX_KEY2);                                                                   \
                                                                                                                       \
        uint32_t v = READ__REG(target, CTLR);                                                                          \
    }
//return !(v & CH32V3XX_FMC_CTL_CH32_FASTUNLOCK); \


#define ch32v3x_wait_not_busy()                                                                                        \
    {                                                                                                                  \
        while (1)                                                                                                      \
        {                                                                                                              \
            uint32_t s = READ__REG(t, STATR);                                                                          \
            if (!(s & CH32V3XX_FMC_STAT_BUSY))                                                                         \
                break;                                                                                                 \
        }                                                                                                              \
    }

#define ch32v3x_wait_not_wr_busy()                                                                                     \
    {                                                                                                                  \
        while (1)                                                                                                      \
        {                                                                                                              \
            uint32_t s = READ__REG(t, STATR);                                                                          \
            if (!(s & CH32V3XX_FMC_STAT_WR_BUSY))                                                                      \
                break;                                                                                                 \
        }                                                                                                              \
    }

/*
 */
#define ch32v3x_ctl_set(bits)                                                                                          \
    {                                                                                                                  \
        uint32_t v = READ__REG(t, CTLR);                                                                               \
        v |= bits;                                                                                                     \
        WRITE__REG(->t, CTLR, v);                                                                                      \
    }

/*
 */
#define ch32v3x_ctl_clear(bits)                                                                                        \
    {                                                                                                                  \
        uint32_t v = READ__REG(t, CTLR);                                                                               \
        v &= ~bits;                                                                                                    \
        WRITE__REG(->t, CTLR, v);                                                                                      \
    }

/**
 */
#define ch32v3x_stat_set(bits)                                                                                         \
    {                                                                                                                  \
        uint32_t v = READ__REG(t, STATR);                                                                              \
        v |= bits;                                                                                                     \
        WRITE__REG(0, STATR, v);                                                                                       \
    }

typedef void (*rv32_end)(void);

#define EXIT(x)  __asm__("ebreak")

#define EXIT2(x)                                                                                                        \
    {                                                                                                                  \
        rv32_end end = (rv32_end)0x20000000;                                                                           \
        end();                                                                                                         \
    }
