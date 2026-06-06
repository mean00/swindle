/*
 *  (C) 2021 MEAN00 fixounet@free.fr
 *  See license file
 */

/**
 * @file lnDebug.cpp
 * @brief Logging utilities for the hosted debugger build.
 *
 * Provides timestamped and plain logging functions used by both
 * the C/C++ debugger core and the Rust BMP logger interface.
 */

#include "stdarg.h"
#include "stdint.h"
#include "stdio.h"
#include "string.h"

#include <unistd.h>

#include <QElapsedTimer>

#define PREFIX_BUFFER_SIZE 20
#define OUTER_BUFFER_SIZE 512

static QElapsedTimer tickTimer;
static bool tickTimerInitialized = false;
static uint32_t originalTick = 0;

/**
 * @brief Get the current tick count (milliseconds since first call).
 * @return Tick count in milliseconds.
 */
uint32_t getTick()
{
    if (!tickTimerInitialized)
    {
        tickTimer.start();
        tickTimerInitialized = true;
    }
    return static_cast<uint32_t>(tickTimer.elapsed());
}

/**
 * @brief Internal logger: print a timestamped formatted message.
 * @param fmt  printf-style format string.
 * @param args Variable argument list.
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

/**
 * @brief Internal logger: print a plain formatted message.
 * @param fmt  printf-style format string.
 * @param args Variable argument list.
 */
static void LoggerInternal(const char *fmt, va_list &args)
{
    static char buffer[PREFIX_BUFFER_SIZE + OUTER_BUFFER_SIZE + 1];
    vsnprintf(buffer, OUTER_BUFFER_SIZE, fmt, args);
    buffer[OUTER_BUFFER_SIZE + PREFIX_BUFFER_SIZE] = 0;
    printf("%s", buffer);
}

/**
 * @brief Log a formatted message (C-linkage, no timestamp).
 * @param fmt  printf-style format string.
 */
extern "C" void Logger_C(const char *fmt, ...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternal(fmt, va);
    va_end(va);
}

/**
 * @brief Log a formatted message with a timestamp and a raw data suffix.
 * @param n    Number of bytes of raw data to append.
 * @param fmt  printf-style format string.
 */
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

/**
 * @brief Log a formatted message (no timestamp).
 * @param fmt  printf-style format string.
 */
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
 * @brief Initialise the logger (records the starting tick).
 */
void LoggerInit()
{
    originalTick = getTick();
}

/**
 * @brief Log a formatted message with a [ss:mmm] timestamp prefix (C-linkage).
 * @param fmt  printf-style format string.
 */
extern "C" void bmploggerh(const char *fmt...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternalTimeStamp(fmt, va);
    va_end(va);
}

/**
 * @brief Log a formatted message without timestamp (C-linkage).
 * @param fmt  printf-style format string.
 */
extern "C" void bmplogger(const char *fmt...)
{
    if (!fmt[0])
        return;
    va_list va;
    va_start(va, fmt);
    LoggerInternal(fmt, va);
    va_end(va);
}

/**
 * @brief Log a raw binary buffer, sanitising non-printable characters (C-linkage).
 * @param n  Number of bytes in the buffer.
 * @param a  Pointer to the buffer data.
 */
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