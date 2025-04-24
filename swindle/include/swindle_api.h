//
#pragma once
#ifdef __cplusplus
extern "C"
{
#endif
#include "stdint.h"
    bool bmp_settings_set(const char *set, uint32_t value);
    bool bmp_settings_unset(const char *set);
    uint32_t bmp_settings_get_or_default(const char *key, uint32_t default_value);

#ifdef __cplusplus
}
#endif
