/*

 */
 extern "C"
 {
#include "version.h"
#include "gdb_packet.h"
#include "gdb_main.h"
#include "target.h"
#include "gdb_packet.h"
#include "morse.h"
#include "general.h"

}
static char tmpBuffer[128]; // ???
extern "C" int vasprintf_(char **strp, const char *fmt, va_list ap)
{
  // this is suboptimal
  vsnprintf((char *)tmpBuffer,127,fmt,ap);
  int sz=strlen((char *)tmpBuffer);
  char *n=(char *)malloc(sz+1);
  strcpy(n,tmpBuffer);
  *strp=n;
  return sz;
}
