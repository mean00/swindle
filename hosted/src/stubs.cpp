/**
 * @file stubs.cpp
 * @brief Stub implementations for functions expected by the BMP core.
 *
 * Many of these are no-ops or assertion failures, as the hosted build
 * does not support the corresponding hardware features (SPI, SWD, etc.).
 */

#include "stdint.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
extern "C"
{
#include "bmp_remote.h"
#include "src/remote.h"
}

#define UNIMPLEMENTED_STUB()                                                                                           \
    {                                                                                                                  \
        printf("UNIMPLEMENTED :%s\n", __PRETTY_FUNCTION__);                                                            \
        xAssert(0);                                                                                                    \
    }

/**
 * @brief Assertion handler: prints failure message and exits.
 * @param a  Assertion expression string (unused).
 */
void xAssert(int a)
{
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    printf("******* FAILURE *********\n");
    exit(0);
}

/**
 * @brief Stub: SPI init (not supported in hosted build).
 * @param bus  SPI bus identifier.
 * @return false.
 */
extern "C" bool remote_v3_spi_init(const spi_bus_e bus)
{
    return false;
}

/**
 * @brief Stub: SPI deinit (not supported in hosted build).
 * @param bus  SPI bus identifier.
 * @return false.
 */
extern "C" bool remote_v3_spi_deinit(const spi_bus_e bus)
{
    return false;
}

/**
 * @brief Stub: SPI chip select (not supported in hosted build).
 * @param device_select  Device select value.
 * @return false.
 */
extern "C" bool remote_v3_spi_chip_select(uint8_t device_select)
{
    return false;
}

/**
 * @brief Stub: SPI transfer (not supported in hosted build).
 * @param bus    SPI bus identifier.
 * @param value  Byte to send.
 * @return 0.
 */
extern "C" uint8_t remote_v3_spi_xfer(spi_bus_e bus, uint8_t value)
{
    return 0;
}

/**
 * @brief Stub: RTT send data to host (no-op).
 */
extern "C" void swindle_rtt_send_data_to_host(uint32_t index, uint32_t len, const uint8_t *data)
{
}

/**
 * @brief Stub: RTT query room available to host.
 * @param index  Channel index.
 * @return 0.
 */
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t index)
{
    return 0;
}

/**
 * @brief Stub: set NRST pin (no-op).
 * @param assert  true to assert reset, false to release.
 */
extern "C" void platform_nrst_set_val_internal(bool assert)
{
}

/**
 * @brief Stub: begin BMP I/O session (no-op).
 */
extern "C" void bmp_io_begin_session()
{
}

/**
 * @brief Stub: end BMP I/O session (no-op).
 */
extern "C" void bmp_io_end_session()
{
}

/**
 * @brief Stub: ADIv5 SWD write without check (unimplemented).
 * @param addr  Register address.
 * @param data  Value to write.
 * @return false.
 */
extern "C" bool adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    UNIMPLEMENTED_STUB();
    return 0;
}

/**
 * @brief Stub: ADIv5 SWD read without check (unimplemented).
 * @param addr  Register address.
 * @return 0.
 */
extern "C" uint32_t adiv5_swd_read_no_check(const uint16_t addr)
{
    UNIMPLEMENTED_STUB();
    return 0;
}

/**
 * @brief Fatal error handler: prints message and exits.
 * @param z  Error message string.
 */
extern "C" void do_assert(const char *z)
{
    printf("FATAL ERROR :%s\n", z);
    exit(-1);
}

/**
 * @brief Stub: putchar (no-op).
 */
extern "C" void _putchar(char) {};

/**
 * @brief Stub: set wait state.
 * @param ws  Wait state value.
 */
extern "C" void bmp_set_wait_state_c(unsigned int ws)
{
    printf("Stubbed : set ws %d\n", ws);
}

/**
 * @brief Stub: get wait state.
 * @return 5 (hardcoded stub value).
 */
extern "C" unsigned int bmp_get_wait_state_c(void)
{
    printf("Stubbed : get ws %d\n", 5);
    return 5;
}

/**
 * @brief Stub: GDB interface init.
 * @return 0.
 */
extern "C"
{
    int gdb_if_init(void)
    {
        return 0;
    }
}

extern "C"
{
    extern void remote_pin_set(uint8_t p, uint8_t s);
    extern void remote_pin_direction(uint8_t p, uint8_t is_direction);
    extern bool remote_pin_get(uint8_t p);

    /**
     * @brief Set a BMP pin state via remote protocol.
     * @param pin    Pin number.
     * @param state  Pin state (0/1).
     */
    void bmp_pin_set(uint8_t pin, uint8_t state)
    {
        remote_pin_set(pin, state);
    }

    /**
     * @brief Get a BMP pin state via remote protocol.
     * @param pin  Pin number.
     * @return true if pin is high.
     */
    bool bmp_pin_get(uint8_t pin)
    {
        return remote_pin_get(pin);
    }

    /**
     * @brief Set a BMP pin direction via remote protocol.
     * @param pin       Pin number.
     * @param is_write  true for output, false for input.
     */
    void bmp_pin_direction(uint8_t pin, uint8_t is_write)
    {
        remote_pin_direction(pin, is_write);
    }

    // This shouold be in platform.c but we put them here to minimize the change
    // in blackmagic source tree

    /**
     * @brief Send a generic pin command over the remote protocol.
     * @param code   Command code ('W' for set, 'X' for direction, 'w' for get).
     * @param pin    Pin number.
     * @param value  Value to write (for set/direction).
     */
    void remote_pin_gen(uint8_t code, uint8_t pin, uint8_t value)
    {
        uint8_t buffer[REMOTE_MAX_MSG_SIZE] = {REMOTE_SOM, REMOTE_GEN_PACKET, code, pin, value, REMOTE_EOM, 0};
        platform_buffer_write(buffer, 6);
        int length = platform_buffer_read(buffer, REMOTE_MAX_MSG_SIZE);
        if (length < 1)
        {
            DEBUG_ERROR("Invalid pin set reply\n");
            return; // false;
        }
        bool r = buffer[0] == REMOTE_RESP_OK;
        if (!r)
        {
            DEBUG_ERROR(" pin set error\n");
        }
        // return r;
    }

    /**
     * @brief Set pin direction via remote protocol.
     * @param pin        Pin number.
     * @param is_write   true for output, false for input.
     */
    void remote_pin_direction(uint8_t pin, uint8_t is_write)
    {
        remote_pin_gen('X', pin, is_write);
    }

    /**
     * @brief Set pin state via remote protocol.
     * @param pin    Pin number (0 = clk, 1 = io).
     * @param value  Pin state (0/1).
     */
    void remote_pin_set(uint8_t pin, uint8_t value) // pin 0 = clk, pin 1 = io
    {
        remote_pin_gen('W', pin, value);
    }

    /**
     * @brief Get pin state via remote protocol.
     * @param pin  Pin number (0 = clk, 1 = io).
     * @return true if pin is high.
     */
    bool remote_pin_get(uint8_t pin) // pin 0 = clk, pin 1 = io
    {
        uint8_t buffer[REMOTE_MAX_MSG_SIZE] = {REMOTE_SOM, REMOTE_GEN_PACKET, 'w', pin, REMOTE_EOM, 0};
        platform_buffer_write(buffer, 5);
        int length = platform_buffer_read(buffer, REMOTE_MAX_MSG_SIZE);
        if (length < 1)
        {
            DEBUG_ERROR("Invalid pin set reply\n");
            return false;
        }
        bool r = buffer[0] == REMOTE_RESP_OK;
        if (!r)
        {
            DEBUG_ERROR(" pin set error\n");
            xAssert(0);
        }
        else
        {
            // this is hackish
            switch (buffer[2])
            {
            case '0':
                r = 0;
                break;
            case '1':
                r = 1;
                break;
            default:
                xAssert(0);
            }
        }
        return r;
    }

    /**
     * @brief Stub: RISC-V JTAG DTM handler (no-op).
     * @param dev_index  Device index.
     */
    void riscv_jtag_dtm_handler(const uint8_t dev_index)
    {
    }

    /**
     * @brief Stub: get target voltage.
     * @return 0.0.
     */
    float bmp_get_target_voltage_c()
    {
        return 0.0;
    }

    /**
     * @brief Stub: get free heap size.
     * @return 33.
     */
    size_t xPortGetFreeHeapSize(void)
    {
        return 33;
    }

    /**
     * @brief Stub: get minimum ever free heap size.
     * @return 2.
     */
    size_t xPortGetMinimumEverFreeHeapSize(void)
    {
        return 2;
        ;
    }
}

/**
 * @brief Stub: software system reset (no-op).
 */
void lnSoftSystemReset()
{
}

/**
 * @brief Stub: ST-Link ADIv5 DP init (unimplemented).
 */
extern "C" void stlink_adiv5_dp_init()
{
    xAssert(0);
}

/**
 * @brief Stub: DAP run command (unimplemented).
 * @return false.
 */
extern "C" bool dap_run_cmd(const void *request_data, size_t request_length, void *response_data,
                            size_t response_length)
{
    xAssert(0);
    return false;
}

/**
 * @brief Stub: RISC-V DM read (unimplemented).
 * @param adr    Register address.
 * @param value  Output value.
 * @return false.
 */
extern "C" bool bmp_rv_dm_read_c(uint8_t adr, uint32_t *value)
{
    xAssert(0);
    return false;
}

/**
 * @brief Stub: RISC-V DM write (unimplemented).
 * @param adr    Register address.
 * @param value  Value to write.
 * @return false.
 */
extern "C" bool bmp_rv_dm_write_c(uint8_t adr, uint32_t value)
{
    xAssert(0);
    return false;
}

/**
 * @brief Stub: RISC-V DM reset (unimplemented).
 * @return false.
 */
extern "C" bool bmp_rv_dm_reset_c()
{
    xAssert(0);
    return false;
}

/**
 * @brief Stub: SW-DP init (no-op).
 */
extern "C" void swdptap_init(void)
{
}

/**
 * @brief Stub: RISC-V JTAG DMI read (unimplemented).
 * @return false.
 */
extern "C" bool remote_v4_riscv_jtag_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    xAssert(0);
    return false;
}

/**
 * @brief Stub: RISC-V JTAG DMI write (unimplemented).
 * @return false.
 */
extern "C" bool remote_v4_riscv_jtag_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    xAssert(0);
    return false;
}

/**
 * @brief Stub: ADIv6 AP read (unimplemented).
 */
extern "C" void remote_v4_adiv6_ap_read()
{
    xAssert(0);
}

/**
 * @brief Stub: ADIv6 AP write (unimplemented).
 */
extern "C" void remote_v4_adiv6_ap_write()
{
    xAssert(0);
}

/**
 * @brief Stub: ADIv6 memory read bytes (unimplemented).
 */
extern "C" void remote_v4_adiv6_mem_read_bytes()
{
    xAssert(0);
}

/**
 * @brief Stub: ADIv6 memory write bytes (unimplemented).
 */
extern "C" void remote_v4_adiv6_mem_write_bytes()
{
    xAssert(0);
}

/**
 * @brief Stub: nRF51 CTRL AP probe.
 * @param ap  Access port (unused).
 * @return false.
 */
extern "C" bool x_nrf51_ctrl_ap_probe(adiv5_access_port_s *ap)
{
    (void)ap;
    return false;
}

/**
 * @brief Stub: set logger (no-op).
 */
extern "C" void setLogger(void *)
{
}

// -- eof --

/**
 * @brief Stub: USB CDC logger (no-op).
 */
extern "C" void usbCdc_Logger(int n, const char *data)
{
}

/**
 * @brief Stub: get FreeRTOS debug info.
 * @return NULL.
 */
extern "C" const uint32_t *lnGetFreeRTOSDebug()
{
    return NULL;
}
//
extern "C" void bmp_gpio_reset_c();
extern "C" void bmp_gpio_reset()
{
    bmp_gpio_reset_c();
}
void ln_set_connected_state(bool connected)
{
}
