/**
 * @file hosted_constants.h
 * @brief Common constants for swindle_hosted PC debugger.
 */

#pragma once
#include <stdint.h>

// Networking constants
static constexpr uint16_t DEFAULT_GDB_TCP_PORT = 2000;
static constexpr uint32_t EVENT_POLL_TIMEOUT_MS = 20;

// USB Vendor & Product IDs
static constexpr uint16_t USB_VID_OPENMOKO = 0x1d50;
static constexpr uint16_t USB_PID_BMP_CLASSIC = 0x6018;
static constexpr uint16_t USB_PID_SWINDLE_LN = 0x6030;    // GD32 / CH32 "Swindle"
static constexpr uint16_t USB_PID_SWINDLE_PICO = 0x6050;  // RP2040 "pSwindle"

// Serial communication configuration
static constexpr int SERIAL_BAUD_RATE = 115200;
static constexpr int SERIAL_WAIT_TIMEOUT_MS = 10;
static constexpr int SERIAL_BUFFER_SIZE = 512;
static constexpr int PLATFORM_BUFFER_SIZE = 512;

// Serial RPC framing protocol
static constexpr uint8_t FRAME_START_DELIMITER = '&';
static constexpr uint8_t FRAME_END_DELIMITER = '#';
static constexpr uint8_t FRAME_RESP_OK = 'K';
static constexpr uint8_t FRAME_RESP_ERROR = 'E';

// Serial error return codes
static constexpr int SERIAL_READ_ERROR_IO = -1;
static constexpr int SERIAL_READ_ERROR_OVERFLOW = -2;
