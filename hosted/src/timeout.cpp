
extern "C"
{
#include "general.h"
}
#include <QElapsedTimer>
static QElapsedTimer timer;
extern "C"
{
    void platform_timeout_init()
    {
        timer.start();
    }
    void platform_timeout_set(platform_timeout_s *const t, uint32_t ms)
    {
        t->time = timer.elapsed() + ms;
    }

    bool platform_timeout_is_expired(const platform_timeout_s *const t)
    {
        uint32_t now = timer.elapsed();
        if (now > t->time)
            return true;
        return false;
    }
}
