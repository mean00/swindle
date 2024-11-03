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
 *
 * CHIPID table
 *	CH32V303CBT6: 0x303305x4
 *	CH32V303RBT6: 0x303205x4
 *	CH32V303RCT6: 0x303105x4
 *	CH32V303VCT6: 0x303005x4
 *	CH32V305FBP6: 0x305205x8
 *	CH32V305RBT6: 0x305005x8
 *	CH32V307WCU6: 0x307305x8
 *	CH32V307FBP6: 0x307205x8
 *	CH32V307RCT6: 0x307105x8
 *	CH32V307VCT6: 0x307005x8
 */

#include "general.h"
#include "riscv_debug.h"
#include "target.h"
#include "target_internal.h"

#include "exception.h"
#include "flashstub/ch32v3x_erase.stub"
#include "flashstub/ch32v3x_write.stub"
#include "riscv_debug.h"
// tmp
#include "gdb_packet.h"
//
#define debug DEBUG_ERROR

#define RAM_ADDRESS 0x20000000
#define FLASH_OFFSET 0x08000000

#define STUB_CODE_LOCATION_ERASE (RAM_ADDRESS + 4 * 1024)
#define STUB_CODE_LOCATION_WRITE (STUB_CODE_LOCATION_ERASE + 512)
#define STUB_STACK_LOCATION (RAM_ADDRESS + 2 * 1024)
#define STUB_STACKEND_LOCATION (RAM_ADDRESS + 4 * 1024 - 16)
#define STUB_DATA_LOCATION (RAM_ADDRESS + 6 * 1024)

// #define VERIFY 1

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
} ch32_flash_s;

#define CH32V3XX_OPTION_ADDRESS 0x1ffff800
#define CH32V3XX_FLASH_CONTROLLER_ADDRESS 0x40022000
#define CH32V3XX_UID1 0x1ffff7e8      // Low bits of UUID
#define CH32V3XX_FMC_CTL_PG (1 << 0)  // program command
#define CH32V3XX_FMC_CTL_PER (1 << 1) // page erase command
#define CH32V3XX_FMC_CTL_CH32_OBPG (1 << 4)
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

#define READ_FLASH_REG(target, reg)                                                                                    \
    target_mem32_read32(target, CH32V3XX_FLASH_CONTROLLER_ADDRESS + offsetof(ch32_flash_s, reg))
#define WRITE_FLASH_REG(target, reg, value)                                                                            \
    target_mem32_write32(target, CH32V3XX_FLASH_CONTROLLER_ADDRESS + offsetof(ch32_flash_s, reg), value)

const command_s ch32v3x_cmd_list[] = {{"", NULL, ""}};

static bool ch32v3x_flash_erase_flashstub(target_flash_s *flash, target_addr_t addr, size_t len);
static bool ch32v3x_flash_write_flashstub(target_flash_s *flash, target_addr_t dest, const void *src, size_t len);
static bool ch32v3x_flash_prepare_flashstub(target_flash_s *flash);
static bool ch32v3x_flash_done_flashstub(target_flash_s *flash);

/*
 */
static bool ch32v3x_fast_unlock(target_s *target)
{
    uint32_t ctl = READ_FLASH_REG(target, CTLR);
    if (!(ctl & CH32V3XX_FMC_CTL_LK)) // already unlocked
        return true;
    // send unlock sequence
    WRITE_FLASH_REG(target, KEYR, CH32V3XX_KEY1);
    WRITE_FLASH_REG(target, KEYR, CH32V3XX_KEY2);

    // send fast unlock sequence
    WRITE_FLASH_REG(target, MODEKEYR, CH32V3XX_KEY1);
    WRITE_FLASH_REG(target, MODEKEYR, CH32V3XX_KEY2);

    uint32_t v = READ_FLASH_REG(target, CTLR);
    return !(v & CH32V3XX_FMC_CTL_CH32_FASTUNLOCK);
}

/**
 * @brief
 *
 * @param flash
 * @param addr
 * @param len
 * @return true
 * @return false
 */
static bool ch32v3x_flash_erase_flashstub(target_flash_s *flash, target_addr_t addr, size_t len)
{
    addr |= FLASH_OFFSET;
    while (len)
    {
        uint32_t chunk = len;
        if (chunk > 1024)
            chunk = 1024;
        if (!riscv32_run_stub(flash->t, STUB_CODE_LOCATION_ERASE, addr, chunk, 0, STUB_STACKEND_LOCATION))
            return false;
        addr += chunk;
        len -= chunk;
    }
    return true;
}

/*
    download the 2 flash stub once in ram so we can execute them later on
 */
static bool ch32v3x_flash_prepare_flashstub(target_flash_s *flash)
{
    target_mem32_write(flash->t, STUB_CODE_LOCATION_ERASE, ch32v3x_erase_bin, sizeof(ch32v3x_erase_bin));
    target_mem32_write(flash->t, STUB_CODE_LOCATION_WRITE, ch32v3x_write_bin, sizeof(ch32v3x_write_bin));
    ch32v3x_fast_unlock(flash->t);
    return true;
}

/*
    Make sure the cpu is stopped
 */
static bool ch32v3x_flash_done_flashstub(target_flash_s *flash)
{
    flash->t->halt_request(flash->t);
    return true;
}

/**
    write a chunk of code in flash/rram through flashstub
 */
static bool ch32v3x_flash_write_flashstub(target_flash_s *flash, target_addr_t dest, const void *srcx, size_t len)
{
    dest |= FLASH_OFFSET;
    uint32_t addr = dest;
    while (len)
    {
        uint32_t chunk = len;
        if (chunk > 1024)
            chunk = 1024;
        target_mem32_write(flash->t, STUB_DATA_LOCATION, srcx, chunk);
        if (!riscv32_run_stub(flash->t, STUB_CODE_LOCATION_WRITE, addr, STUB_DATA_LOCATION, chunk,
                              STUB_STACKEND_LOCATION))
            return false;
        addr += chunk;
        len -= chunk;
        srcx += chunk;
    }
    return true;
}

/*
 */
static void ch32v3x_add_flash(target_s *target, uint32_t addr, size_t length, size_t erasesize, size_t writesize)
{
    target_flash_s *flash = calloc(1, sizeof(*flash));
    if (!flash)
    { /* calloc failed: heap exhaustion */
        DEBUG_ERROR("calloc: failed in %s\n", __func__);
        return;
    }
    flash->start = addr;
    flash->length = length;
    flash->blocksize = erasesize;
    flash->writesize = writesize;
    flash->erase = ch32v3x_flash_erase_flashstub;
    flash->write = ch32v3x_flash_write_flashstub;
    flash->prepare = ch32v3x_flash_prepare_flashstub;
    flash->done = ch32v3x_flash_done_flashstub;
    flash->erased = 0xff;
    target_add_flash(target, flash);
}
#define MET_MEMORY(f, r)                                                                                               \
    {                                                                                                                  \
        *flash = f;                                                                                                    \
        *ram = r;                                                                                                      \
    }
/**
 *
 */
static void decode_memory_layout(uint32_t layout, uint32_t *flash, uint32_t *ram)
{
    switch (layout)
    {
    case 0:
    case 1:
        MET_MEMORY(192, 128);
        break;
    case 2:
    case 3:
        MET_MEMORY(224, 96);
        break;
    case 4:
    case 5:
        MET_MEMORY(256, 64);
        break;
    case 6:
        // that one does not exist with older chip only when
        //  the penultimate sixth digit of the lot number is not zero.
        // SET_MEMORY(128, 192); assume older chip
        // break;
        DEBUG_WARN("WARNING NEWER MEMORY LAYOUT, ASSUMING 228k flash 32k ram \n");
    case 7:
        MET_MEMORY(228, 32);
        break;
    default:
        DEBUG_WARN("\t???\n");
        *flash = 128; // ?
        *ram = 32;
        break;
    }
}
/*
    Identify ch32vxx
*/
bool ch32v3xx_probe(target_s *target)
{
    uint32_t flash_size = 0;
    uint32_t ram_size = 0;
    size_t erase_size = 256;
    size_t write_size = 256;

    const uint32_t chipid = target_mem32_read32(target, CH32VX_CHIPID);

    const uint16_t family = (chipid & CH32VX_CHIPID_FAMILY_MASK) >> CH32VX_CHIPID_FAMILY_OFFSET;
    bool detect_size = false;
    switch (family)
    {
    case 0x203U:
        detect_size = false;
        target->driver = "CH32V203";
        break;
    case 0x303U:
        detect_size = true;
        target->driver = "CH32V303";
        break;
    case 0x305U:
        detect_size = true;
        target->driver = "CH32V305";
        break;
    case 0x307U:
        detect_size = true;
        target->driver = "CH32V307";
        break;
    default:
        DEBUG_WARN("Unknown CH32V family 0x%x\n", family);
        return false;
        break;
    }
    DEBUG_WARN("CH32V family %s\n", target->driver);

    target->part_id = chipid;

    if (detect_size)
    {
        // this is the user option byte
        // there are 2 variants of that, depending on the chip Identity.
        // for now we use the "old" one
        uint32_t user_ram_size, user_flash_size;
        uint32_t user = target_mem32_read32(target, 0x1ffff800);
        DEBUG_WARN("CH32V User option byte 0x%x, up=0x%x\n", user, user >> 16);
        uint32_t layout = (user >> (16 + 5)) & 0x7;
        DEBUG_WARN("CH32V User memory layout 0x%x\n User byte ", layout);
        decode_memory_layout(layout, &user_flash_size, &user_ram_size);
        DEBUG_WARN("CH32V User memory  %d k flash, %d k ram\n", user_flash_size, user_ram_size);

        uint32_t obr = READ_FLASH_REG(target, OBR); // offset 01xc 32.4.6
        DEBUG_WARN("CH32V OBR Register 0x%x\n", obr);
        obr = (obr >> 7) & 7; // SRAM_CODE_MODE, the doc is incorrect i think, it says >>8
        DEBUG_WARN("CH32V OBR Memory Layout 0x%x\n", obr);
        decode_memory_layout(layout, &flash_size, &ram_size);
        DEBUG_WARN("CH32V OBR %d k flash, %d k ram\n", flash_size, ram_size);
        gdb_outf("\tDetected %x chip with %d k flash, %d k ram\n", family, flash_size, ram_size);
    }
    else // only deal with 203 for the moment
    {
        switch (chipid)
        {
#define CH32_HARDCODED(x, y, z)                                                                                        \
    case x:                                                                                                            \
        flash_size = y;                                                                                                \
        ram_size = z;                                                                                                  \
        break;
            CH32_HARDCODED(0x20310500, 64, 20)
        default:
            flash_size = 64; // ?
            ram_size = 20;
            break;
        }
    }
    DEBUG_WARN("Finally, using CH32V flash %d kB, ram %d kB\n", flash_size, ram_size);
    target_mem_map_free(target);
    target_add_ram32(target, RAM_ADDRESS, ram_size * 1024U);
    ch32v3x_add_flash(target, 0x0, (size_t)flash_size * 1024U, erase_size, write_size);
    target_add_commands(target, ch32v3x_cmd_list, target->driver);
    return true;
}
/**
 * \briegf unlock option bytes
 */
static bool ch32v3xx_unlock_option_bytes(target_s *target)
{
    WRITE_FLASH_REG(target, KEYR, CH32V3XX_KEY1);
    WRITE_FLASH_REG(target, KEYR, CH32V3XX_KEY2);
    // now unlock OB
    WRITE_FLASH_REG(target, OBKEYR, CH32V3XX_KEY1);
    WRITE_FLASH_REG(target, OBKEYR, CH32V3XX_KEY2);

    return true;
}
/**
 *
 *
 */
bool ch32v3xx_write_user_byte(target_s *target, uint8_t user)
{
    ch32v3xx_unlock_option_bytes(target);

    uint32_t ctl = READ_FLASH_REG(target, CTLR);
    WRITE_FLASH_REG(target, CTLR, ctl | CH32V3XX_FMC_CTL_CH32_OBPG);
    // 16 bits write only!
    // hw will put the correct inverted value
    uint16_t user16 = (uint16_t)user | ((uint16_t)user << 8);
    target_mem32_write(target, CH32V3XX_OPTION_ADDRESS + 2, &user16, 2);
    while (1)
    {
        uint32_t stat = READ_FLASH_REG(target, STATR);
        if (!(stat & CH32V3XX_FMC_STAT_BUSY))
            break;
    }
    // and done
    WRITE_FLASH_REG(target, CTLR, ctl);
    return true;
}
/**
 *
 */
uint8_t ch32v3xx_read_user_byte(target_s *target) // eof
{
    uint32_t user32 = target_mem32_read32(target, CH32V3XX_OPTION_ADDRESS);
    user32 >>= 16; // start at offset 2
    user32 &= 0xff;
    return (uint8_t)user32;
}
