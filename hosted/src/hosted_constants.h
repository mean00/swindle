/**
 * @file hosted_constants.h
 * @brief Common constants for swindle_hosted PC debugger.
 */

#pragma once
#include <stdint.h>

// Networking constants
static constexpr uint16_t DEFAULT_GDB_TCP_PORT = 2000;
static constexpr uint32_t EVENT_POLL_TIMEOUT_MS = 20;

// Runner slot bits (the layout rs_socketrunner uses; slot 0, i.e. no shift).
static constexpr uint32_t RUNNER_UP = 1u << 30;
static constexpr uint32_t RUNNER_DOWN = 1u << 31;
static constexpr uint32_t RUNNER_DISCONNECTED = 1u << 1;
static constexpr uint32_t RUNNER_CAN_WRITE = 1u << 3;
static constexpr uint32_t RUNNER_SLOT_MASK = 0x1Fu;

// Network probe (swindle_hosted_net): time allowed to reach the probe's own TCP
// server, and how long a relayed request waits for its reply before the poll
// tick picks it up.
static constexpr int PROBE_CONNECT_TIMEOUT_MS = 3000;
static constexpr int RELAY_REPLY_WAIT_MS = 50;

// USB Vendor & Product IDs
static constexpr uint16_t USB_VID_OPENMOKO = 0x1d50;
static constexpr uint16_t USB_PID_BMP_CLASSIC = 0x6018;
static constexpr uint16_t USB_PID_SWINDLE_LN = 0x6030;    // GD32 / CH32 "Swindle"
static constexpr uint16_t USB_PID_SWINDLE_PICO = 0x6050;  // RP2040 "pSwindle"

// Serial communication configuration
static constexpr int SERIAL_BAUD_RATE = 115200;
static constexpr int SERIAL_WAIT_TIMEOUT_MS = 10;
// Only used when QSerialPort had to buffer a partial write; waiting for the
// write notification with an empty buffer just burns the whole timeout.
static constexpr int SERIAL_WRITE_DRAIN_TIMEOUT_MS = 50;
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
