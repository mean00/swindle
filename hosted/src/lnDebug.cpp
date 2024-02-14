/*
 *  (C) 2021 MEAN00 fixounet@free.fr
 *  See license file
 */

#include "stdarg.h"
#include "stdint.h"
#include "stdio.h"
#include "string.h"

#include <time.h>
#include <unistd.h>

#define PREFIX_BUFFER_SIZE 20
#define OUTER_BUFFER_SIZE 512

static uint32_t originalTick = 0;

uint32_t getTick()
{
    struct timespec ts;
    unsigned theTick = 0U;
    clock_gettime(CLOCK_REALTIME, &ts);
    theTick = ts.tv_nsec / 1000000;
    theTick += ts.tv_sec * 1000;
    return theTick;
}

/**

*/
static void LoggerInternalTimeStamp(const char *fmt, va_list &args)
{
    if (!originalTick)
        originalTick = getTick();
    uint32_t tick = getTick() - originalTick;
    static char buffer[PREFIX_BUFFER_SIZE + OUTER_BUFFER_SIZE + 1];

    int ss = tick / 1000;
    int ms = tick - (ss * 1000);

    sprintf(buffer, "[%02d:%03d]", ss, ms);
    int ln = strlen(buffer);

    vsnprintf(buffer + ln, OUTER_BUFFER_SIZE, fmt, args);
    buffer[OUTER_BUFFER_SIZE + PREFIX_BUFFER_SIZE] = 0;
    printf("%s", buffer);
}
static void LoggerInternal(const char *fmt, va_list &args)
{
    static char buffer[PREFIX_BUFFER_SIZE + OUTER_BUFFER_SIZE + 1];
    vsnprintf(buffer, OUTER_BUFFER_SIZE, fmt, args);
    buffer[OUTER_BUFFER_SIZE + PREFIX_BUFFER_SIZE] = 0;
    printf("%s", buffer);
}
extern "C" void Logger_C(const char *fmt, ...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternal(fmt, va);
    va_end(va);
}
extern "C" void Logger2(int n, const char *fmt)
{
    if (!fmt[0])
        return;
    if (!originalTick)
        originalTick = getTick();
    uint32_t tick = getTick() - originalTick;
    static char buffer[PREFIX_BUFFER_SIZE + OUTER_BUFFER_SIZE + 1];

    int ss = tick / 1000;
    int ms = tick - (ss * 1000);

    sprintf(buffer, "[%02d:%03d]", ss, ms);
    int ln = strlen(buffer);
    printf("%s", buffer);
    memcpy(buffer, fmt, n);
    buffer[n] = 0;
    printf("%s", buffer);
}

extern "C" void Logger(const char *fmt, ...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternal(fmt, va);
    va_end(va);
}
/**
 *
 */
void LoggerInit()
{
    originalTick = getTick();
}

extern "C" void bmploggerh(const char *fmt...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternalTimeStamp(fmt, va);
    va_end(va);
}
extern "C" void bmplogger(const char *fmt...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternal(fmt, va);
    va_end(va);
}
extern "C" void bmploggern(int n, const char *a)
{
#define MAX_DUMP 2064
    static char bfer[MAX_DUMP];
    while (n)
    {
        int r = n;
        if (r > MAX_DUMP)
            r = MAX_DUMP;
        memcpy(bfer, a, r);
        for (int i = 0; i < r; i++)
        {
            if (bfer[i] < 32)
                bfer[i] = 32;
        }
        bfer[r] = 0;
        bmplogger("<%s>", bfer);
        n -= r;
        a += r;
    }
}
