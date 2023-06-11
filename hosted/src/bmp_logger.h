

#pragma once
#include "stdarg.h"
#include "stdio.h"
extern void bmplogger(const char *a...);
extern void bmploggern(int n, const char *a);
#define QBMPLOG bmplogger
#define QBMPLOGN(sz, data) bmploggern(sz, data);