

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
uint32_t readMem32(uint32_t base, uint32_t offset);
void writeMem32(uint32_t base, uint32_t offset, uint32_t value);
extern target *cur_target;
extern "C" void gdb_putpacket(const char *packet, size_t size);
extern "C" bool target_validate_address_flash_or_ram(target *, uint32_t address);
