

#pragma once

extern "C"
{
  #include "target.h"
  #include "target_internal.h"
  #include "gdb_packet.h"
  #include "lnFreeRTOSDebug.h"
  #include "hex_utils.h"
}
#include "bmp_string.h"
uint32_t readMem32(uint32_t base, uint32_t offset);
void writeMem32(uint32_t base, uint32_t offset,   uint32_t value);
extern target *cur_target;
extern "C" void gdb_putpacket(const char *packet, int size);

#define O(x) allSymbols._debugInfo.x

#include "bmp_symbols.h"
extern AllSymbols allSymbols;
#include "bmp_freertos_tcb.h"
#include "bmp_thread_cache.h"
extern lnThreadInfoCache *threadCache;
extern "C" bool target_validate_address_flash_or_ram(target *,uint32_t address);
