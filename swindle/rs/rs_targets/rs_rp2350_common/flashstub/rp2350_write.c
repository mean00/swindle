#include "rp2350_qmi.h"

int rp2350_flash_write(uint32_t dest, uint32_t src, uint32_t len)
{
    const uint8_t *data = (const uint8_t *)src;
    uint32_t flash_addr = dest & 0x00ffffffU;
    uint32_t remaining = len;

    while (remaining > 0)
    {
        uint32_t page_offset = flash_addr & 0xffU;
        uint32_t chunk = 256U - page_offset;
        if (chunk > remaining)
            chunk = remaining;

        // 1. Write enable
        qmi_cmd(SPI_CMD_WRITE_ENABLE);

        // 2. Page program command + 24-bit address
        qmi_cs_assert();
        qmi_tx8(SPI_CMD_PAGE_PROGRAM);
        qmi_tx8((flash_addr >> 16) & 0xffU);
        qmi_tx8((flash_addr >> 8) & 0xffU);
        qmi_tx8(flash_addr & 0xffU);

        // 3. Stream data
        for (uint32_t i = 0; i < chunk; )
        {
            if (i + 1 < chunk)
            {
                uint16_t word = (uint16_t)data[i] | ((uint16_t)data[i + 1] << 8);
                qmi_tx16(word);
                i += 2;
            }
            else
            {
                qmi_tx8(data[i]);
                i += 1;
            }
        }

        // 4. Finish page transfer
        qmi_wait_not_busy();
        qmi_cs_deassert();

        // 5. Wait for flash internal write cycle to complete
        qmi_wait_ready();

        data += chunk;
        flash_addr += chunk;
        remaining -= chunk;
    }

    return 1;
}
