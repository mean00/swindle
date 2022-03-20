/*
 *  (C) 2021 MEAN00 fixounet@free.fr
 *  See license file
 */

#include "stdio.h"
#include "stdarg.h"
#include "stdarg.h"


extern "C" void Logger_C(const char *fmt,...)
{
  static char buffer[128];

  va_list va;
  va_start(va,fmt);
  vsnprintf(buffer,127,fmt,va);

  buffer[127]=0;
  printf("> %s\n",buffer);
  va_end(va);
}


/**
 *
 * @param fmt
 */
extern "C" void Logger(const char *fmt...)
{
    static char buffer[128];

    if(fmt[0]==0) return;

    va_list va;
    va_start(va,fmt);
    vsnprintf(buffer,127,fmt,va);

    buffer[127]=0;
    printf("> %s\n",buffer);
    va_end(va);
}
/**
 *
 */
void LoggerInit()
{
}
