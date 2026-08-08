
extern "C"
{
#include "general.h"
}
#include <QElapsedTimer>
static QElapsedTimer timer;        // timeout base, restarted by platform_timeout_init()
static QElapsedTimer uptimeTimer;  // program uptime base, started exactly once
extern "C"
{
    void platform_timeout_init()
    {
        timer.start();
        if (!uptimeTimer.isValid())
            uptimeTimer.start();
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
uint32_t lnGetUs()
{
    // Uptime since program start, independent of any timeout-timer restarts.
    if (!uptimeTimer.isValid())
        uptimeTimer.start(); // defensive: establish an epoch even if init was never called
    return static_cast<uint32_t>(uptimeTimer.nsecsElapsed() / 1000);
}
