
extern "C"
{
#include "general.h"

    void platform_timeout_set(platform_timeout_s *const t, uint32_t ms)
    {
    }

    bool platform_timeout_is_expired(const platform_timeout_s *const t)
    {
        return false;
    }
}
