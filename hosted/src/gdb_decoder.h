/**
 * @file gdb_decoder.h
 * @brief On-the-fly human-readable GDB Remote Serial Protocol packet decoder.
 */

#pragma once

#include <stddef.h>
#include <stdint.h>

/**
 * @brief Enable or disable human-readable GDB command decoding.
 */
void gdb_decoder_set_enabled(bool enabled);

/**
 * @brief Check whether GDB command decoding is currently enabled.
 */
bool gdb_decoder_is_enabled();

/**
 * @brief Stream processor: feed incoming raw bytes from GDB client and decode complete commands.
 * @param data  Incoming byte buffer.
 * @param len   Number of bytes in buffer.
 */
void gdb_decode_incoming(const uint8_t *data, uint32_t len);

/**
 * @brief Stream processor: feed outgoing response raw bytes sent to GDB client and decode replies.
 * @param data  Outgoing byte buffer.
 * @param len   Number of bytes in buffer.
 */
void gdb_decode_outgoing(const uint8_t *data, uint32_t len);

/**
 * @brief Decode and print a single null-terminated GDB request packet body (without '$' and '#xx').
 * @param packet  GDB packet body string.
 * @param len     Length of packet string.
 */
void gdb_decode_request_packet(const char *packet, size_t len);

/**
 * @brief Decode and print a single null-terminated GDB response packet body (without '$' and '#xx').
 * @param packet  GDB response body string.
 * @param len     Length of packet string.
 */
void gdb_decode_reply_packet(const char *packet, size_t len);
