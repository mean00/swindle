

#pragma once
#include "stdarg.h"
#include "stdio.h"

/**
 * @brief Log a formatted message (no timestamp prefix).
 * @param a  printf-style format string followed by arguments.
 */
extern "C" void bmplogger(const char *a...);

/**
 * @brief Log a formatted message with a [ss:mmm] timestamp prefix.
 * @param a  printf-style format string followed by arguments.
 */
extern "C" void bmploggerh(const char *a...);

/**
 * @brief Log a raw binary buffer, sanitising non-printable characters.
 * @param n  Number of bytes in the buffer.
 * @param a  Pointer to the buffer data.
 */
extern "C" void bmploggern(int n, const char *a);
#define QBMPLOG bmplogger
#define QBMPLOGH bmploggerh
#define QBMPERROR bmplogger
#define QBMPLOGN(sz, data) bmploggern(sz, data);
