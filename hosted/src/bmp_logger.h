

#pragma once
#include "stdarg.h"
#include "stdio.h"
extern "C" void bmplogger(const char *a...);
extern "C" void bmploggerh(const char *a...);
extern "C" void bmploggern(int n, const char *a);
#define QBMPLOG bmplogger
#define QBMPLOGH bmploggerh
#define QBMPERROR bmplogger
#define QBMPLOGN(sz, data) bmploggern(sz, data);
