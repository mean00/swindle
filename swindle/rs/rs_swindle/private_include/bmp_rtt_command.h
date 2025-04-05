#pragma once

//--
typedef enum
{
    ENABLED,
    ADDRESS,
    POLLING,
} rttField;
//--
typedef struct
{
    uint32_t enabled;
    uint32_t min_address;
    uint32_t max_address;
    uint32_t min_poll_ms;
    uint32_t max_poll_ms;
    uint32_t max_poll_error;
    // read only part
    uint32_t found;
    uint32_t cb_address;
} rttInfo;
