#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "riscv32_flashstub.h"
#include "../ch32v0x_reg.h"

// Redefine the access macros for direct hardware access on the target, but ignore the target param
#define READ_FLASH_REG(target, reg)                                                                                    \
    (*(volatile uint32_t *)(CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg)))
#define WRITE_FLASH_REG(target, reg, value)                                                                            \
    (*(volatile uint32_t *)(CH32V0X_FLASH_CONTROLLER_ADDRESS + offsetof(ch32v0x_flash_s, reg)) = (value))

static inline __attribute__((always_inline)) void ch32v0x_flash_ctl_set(void *target, const uint32_t bits)
{
    WRITE_FLASH_REG(target, CTLR, READ_FLASH_REG(target, CTLR) | bits);
}

static inline __attribute__((always_inline)) bool ch32v0x_flash_wait_not_busy(void *target)
{
    uint32_t status = READ_FLASH_REG(target, STATR);
    while (status & CH32V0X_FMC_STAT_BUSY)
        status = READ_FLASH_REG(target, STATR);

    if (status & CH32V0X_FMC_STAT_BUSY)
        return false;
    return true;
}

static inline __attribute__((always_inline)) bool ch32v0x_flash_check_complete(void *target, uint32_t addr)
{
    const uint32_t status = READ_FLASH_REG(target, STATR);

    /* Writing FLASH_STATR is only allowed with no operation in progress (BSY = 0) */
    WRITE_FLASH_REG(target, STATR, CH32V0X_FMC_STAT_EOP | CH32V0X_FMC_STAT_WRPRTERR);

    if (status & CH32V0X_FMC_STAT_WRPRTERR)
        return false;
    return true;
}

static inline __attribute__((always_inline)) bool target_mem32_write32(void *target, uint32_t addr, uint32_t value)
{
    *(volatile uint32_t *)addr = value;
    return false;
}

static inline __attribute__((always_inline)) uint32_t ch_read_le4(const uint8_t *t)
{
    return *(const uint32_t *)t;
}
#define WRITE_MEM(adr, val)                                                                                            \
    {                                                                                                                  \
        *(uint32_t *)adr = val;                                                                                        \
    }
#define WAIT_BETWEEN_BUFLOAD
#include "../ch32v0x_write.h"

__attribute__((naked, noreturn)) void _start(uint32_t dest, uint32_t src, uint32_t len, uint32_t base_ctlr)
{
    __asm__ volatile("csrci mstatus, 8\n");
    const uint32_t ctlr_bufload = base_ctlr | CH32V0X_FMC_CTL_BUFLOAD;
    uint8_t *data = (uint8_t *)src;

    // Call the shared inner loop, passing NULL for the target pointer
    bool result = ch32v0x_write_inner(NULL, dest, data, len, ctlr_bufload);
    if (result)
    {
        riscv_stub_exit(0);
    }
    else
    {
        riscv_stub_exit(1);
    }
}
