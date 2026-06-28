/**
 * @file lnBMP_pins.h
 * @brief Pin mapping array type definition
 */

#pragma once

/**
 * @brief Logical pin mapping for the debug probe's debug interface.
 *
 * Maps abstract SWD/JTAG signals to numbered slots used by the GPIO layer.
 */
enum lnBMPPins
{
    TTMS_PIN = 0,       /**< JTAG TMS / SWD data (bidirectional). */
    TTDI_PIN = 1,       /**< JTAG TDI (data in). */
    TTDO_PIN = 2,       /**< JTAG TDO (data out). */
    TTCK_PIN = 3,       /**< JTAG/SWD clock. */
    TTRACE_PIN = 4,     /**< SWO trace / Serial Wire Viewer. */
    TSWDIO_PIN = 5,     /**< SWD bidirectional data line. */
    TSWDCK_PIN = 6,     /**< SWD clock line. */
    TRESET_PIN = 7,     /**< Target nRST (reset). */
    TDIRECTION_PIN = 8, /**< Direction control for level shifters. */
};
