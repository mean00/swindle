

/**
 * @file bmp_util.h
 * @brief Miscellaneous utility macros and helpers
 */

#pragma once

extern "C"
{
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "lnFreeRTOSDebug.h"
#include "target.h"
#include "target_internal.h"
}
#include "bmp_string.h"

/**
 * @brief Read a 32-bit word from target memory via the current target.
 * @param base   Base address for the memory read.
 * @param offset Offset added to base for the final address.
 * @return The 32-bit value read from memory.
 */
uint32_t readMem32(uint32_t base, uint32_t offset);

/**
 * @brief Write a 32-bit word to target memory via the current target.
 * @param base   Base address for the memory write.
 * @param offset Offset added to base for the final address.
 * @param value  32-bit value to write.
 */
void writeMem32(uint32_t base, uint32_t offset, uint32_t value);

/** @brief Pointer to the currently selected debug target. */
extern target *cur_target;

/**
 * @brief Send a GDB response packet (wraps gdb_putpacket).
 * @param packet Packet string to send.
 * @param size   Length of the packet.
 */
extern "C" void gdb_putpacket(const char *packet, size_t size);

/**
 * @brief Validate that an address range falls within flash or RAM for the given target.
 * @param t       Target to validate against.
 * @param address Address to check.
 * @return true if the address is valid flash or RAM, false otherwise.
 */
extern "C" bool target_validate_address_flash_or_ram(target *t, uint32_t address);
