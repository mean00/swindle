#include "esprit.h"
#include "lnBMP_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "bmp_pinmode.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
//
#include "driver/spi_master.h"
#include "driver/gpio.h"
//
#include "lnbmp_parity.h"
//
extern spi_device_handle_t esp_spi_handle;
#define RX_TX_BUFFER_SIZE 64
static uint8_t txBuffer[RX_TX_BUFFER_SIZE];
static uint8_t rxBuffer[RX_TX_BUFFER_SIZE];
//
static bool calculate_header_parity(uint8_t h)
{
    // Parity of APnDP, RnW, and Addr bits
    uint8_t bits = (h >> 1) & 0xF;
    return __builtin_popcount(bits) % 2;
}
//
static bool calculate_data_parity(uint32_t d)
{
    return __builtin_popcount(d) % 2;
}
//
static void pack_bits(uint8_t *buf, uint32_t start_bit, uint32_t val, uint8_t len)
{
    for (int i = 0; i < len; i++)
    {
        if (val & (1ULL << i))
        {
            buf[(start_bit + i) / 8] |= (1 << ((start_bit + i) % 8));
        }
    }
}
//
static uint32_t extract_bits(uint8_t *buf, uint32_t start_bit, uint8_t len)
{
    uint32_t res = 0;
    for (int i = 0; i < len; i++)
    {
        if (buf[(start_bit + i) / 8] & (1 << ((start_bit + i) % 8)))
        {
            res |= (1U << i);
        }
    }
    return res;
}
/**
 * @brief [TODO:description]
 *
 * @param addr [TODO:parameter]
 * @param data [TODO:parameter]
 * @return [TODO:return]
 */
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    xAssert(esp_spi_handle);
    const uint32_t is_ap = 0;
    memset(txBuffer, 0, sizeof(txBuffer));

    uint8_t head = make_packet_request(ADIV5_LOW_WRITE, addr);

    // 2. Pack Header (8 bits) + dummy ACK/Trn (4 bits) + Data (32 bits) + Parity (1 bit)
    // We use a helper to shift bits into the tx_pkt.raw buffer LSB-first
    pack_bits(txBuffer, 0, head, 8);
    // Bits 8-11 are turnaround/ignore
    pack_bits(txBuffer, 12, data, 32);
    pack_bits(txBuffer, 44, calculate_data_parity(data), 1);
    // 3 extra = 45+3=48
    spi_transaction_t t = {};
    t.length = 48; // Total bits to clock out
    t.tx_buffer = txBuffer;
    t.rx_buffer = NULL;
    spi_device_polling_transmit(esp_spi_handle, &t);
    return false;
}

/**
        \fn ln_adiv5_swd_read_no_chgck

 */
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr)
{
    const uint32_t is_ap = 0;
    xAssert(esp_spi_handle);
    memset(txBuffer, 0xFF, sizeof(txBuffer)); // Default MOSI to High (Idle)
    memset(rxBuffer, 0x00, sizeof(txBuffer));

    uint8_t head = make_packet_request(ADIV5_LOW_READ, addr);
    txBuffer[0] = head;

    // 2. Perform Full-Duplex SPI Transaction
    // We clock out ~48 bits to ensure we cover Header(8) + Trn(1) + ACK(3) + Data(32) + Parity(1)
    spi_transaction_t t = {};
    t.length = 64;
    t.tx_buffer = txBuffer;
    t.rx_buffer = rxBuffer;
    spi_device_polling_transmit(esp_spi_handle, &t);

    // 3. Extract the 32-bit data from the RX buffer.
    // The data usually starts at bit 12 (8 header + 1 trn + 3 ack).
    return extract_bits(rxBuffer, 12, 32);
}
/**
        \fn ln_adiv5_swd_raw_access

 */
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value)
{
    if ((addr & ADIV5_APnDP) && dp->fault)
        return 0;
    const uint8_t request = make_packet_request(rnw, addr);
    uint8_t ack = SWD_ACK_WAIT;
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 250U);
    uint8_t ff[1] = {0xff};
    spi_transaction_t t = {};
    while (1)
    {
        t.length = 12;
        t.tx_buffer = txBuffer;
        t.rx_buffer = rxBuffer;
        spi_device_polling_transmit(esp_spi_handle, &t);
        uint32_t ack = extract_bits(rxBuffer, 9, 3);
        bool expired = platform_timeout_is_expired(&timeout);
        if (ack == SWD_ACK_OK)
        {
            goto done;
        }

        spi_transaction_t t = {};
        t.length = 2;
        t.tx_buffer = ff;
        t.rx_buffer = rxBuffer;
        spi_device_polling_transmit(esp_spi_handle, &t);

        switch (ack)
        {
        case SWD_ACK_OK:
            xAssert(0);
            break;
        case SWD_ACK_NO_RESPONSE:
            DEBUG_ERROR("SWD access resulted in no response\n");
            dp->fault = ack;
            return 0;

        case SWD_ACK_WAIT:
            if (expired)
            {
                DEBUG_ERROR("SWD access resulted in wait, aborting\n");
                if (dp->fault == SWD_ACK_WAIT)
                    return 0;
                dp->fault = ack; // prevent recursion
                dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
                dp->fault = ack;
                return 0;
            }
            break;
        case SWD_ACK_FAULT:
            if (expired)
            {
                DEBUG_ERROR("SWD access resulted in fault\n");
                dp->fault = ack;
                return 0;
            }
            DEBUG_ERROR("SWD access resulted in fault, retrying\n");
            /* On fault, abort the request and repeat */
            /* Yes, this is self-recursive.. no, we can't think of a better option */
            adiv5_dp_write(dp, ADIV5_DP_ABORT,
                           ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
                               ADIV5_DP_ABORT_STKCMPCLR);
            break;
        default:
            DEBUG_ERROR("SWD access has invalid ack %x\n", ack);
            raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
            break;
        }
    }
done:
    // We are out of read sequence, CLK is low
    if (rnw) // read ****************************************** HERE *************************
    {
        txBuffer[5] = 0;
        txBuffer[6] = 0;
        t.length = 32 + 3 + 1 + 8;
        t.tx_buffer = txBuffer;
        t.rx_buffer = rxBuffer;
        spi_device_polling_transmit(esp_spi_handle, &t);
        uint32_t response = extract_bits(rxBuffer, 0, 32);
        bool parityBit = extract_bits(rxBuffer, 32, 3) & 1;

        uint32_t index = 1;
        bool currentParity = lnOddParity(response);
        if (currentParity != parityBit)
        { /* Give up on parity error */
            dp->fault = 1U;
            DEBUG_ERROR("SWD access resulted in parity error\n");
            raise_exception(EXCEPTION_ERROR, "SWD parity error");
        }
        return response;
    }
    // write
    bool parity = lnOddParity(value);
    // static void pack_bits(uint8_t *buf, uint32_t start_bit, uint32_t val, uint8_t len)
    pack_bits(txBuffer, 0, 3, 2);
    pack_bits(txBuffer, 2, value, 32);
    pack_bits(txBuffer, 34, parity, 1);
    pack_bits(txBuffer, 35, 0, 8);
    spi_device_polling_transmit(esp_spi_handle, &t);
    // zwrite(2, 0x03);
    // zwrite(32, value);
    // zwrite(1, parity);
    // zwrite(8, 0);
    return 0;
    //--
}
/*
 *
 *
 */
extern "C" void ln_raw_swd_write(uint32_t tick, uint32_t value)
{
    xAssert(tick < RX_TX_BUFFER_SIZE * 8);
    spi_transaction_t t = {};
    pack_bits(txBuffer, 0, value, tick);
    t.length = tick;
    t.tx_buffer = txBuffer;
    t.rx_buffer = rxBuffer;
    spi_device_polling_transmit(esp_spi_handle, &t);
}
extern "C" void ln_raw_swd_reset(uint32_t pulses)
{
    // xAssert(tick < RX_TX_BUFFER_SIZE * 8);
    memset(txBuffer, 0xff, sizeof(txBuffer));
    spi_transaction_t t = {};
    t.length = pulses;
    t.tx_buffer = txBuffer;
    t.rx_buffer = NULL;
    spi_device_polling_transmit(esp_spi_handle, &t);
}
// EOF
//
