//
/**
 * @file swindle_api.h
 * @brief Public Swindle debugger API declarations
 */

#pragma once
#ifdef __cplusplus
extern "C"
{
#endif
#include "stdint.h"

    /**
     * @brief Set a debugger setting by name.
     * @param set   Setting name string.
     * @param value Value to assign.
     * @return true on success, false if the setting is unknown.
     */
    bool bmp_settings_set(const char *set, uint32_t value);

    /**
     * @brief Unset (clear) a debugger setting by name.
     * @param set Setting name string.
     * @return true on success, false if the setting is unknown.
     */
    bool bmp_settings_unset(const char *set);

    /**
     * @brief Get a debugger setting value, returning a default if not set.
     * @param key           Setting name string.
     * @param default_value Value to return if the setting is not found.
     * @return The current setting value, or default_value.
     */
    uint32_t bmp_settings_get_or_default(const char *key, uint32_t default_value);

#ifdef __cplusplus
}
#endif
