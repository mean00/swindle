#include "lnBMPArduino.h"
#include <cstdint>
extern "C"
{

#include "ctype.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "general.h"
#include "target.h"

    void gdb_if_putchar(char c, bool flush)
    {
        xAssert(0);
    }
    char gdb_if_getchar_to(uint32_t timeout)
    {
        xAssert(0);
    }

    int vasprintf_(char **strp, const char *fmt, va_list ap)
    {
        xAssert(0);
    }

    void gdb_putpacket_f(const char *const fmt, ...)
    {
        xAssert(0);
    }
    volatile const char *morse_msg = NULL;
    void morse(const char *const msg, const bool repeat)
    {
    }
}
// EOF
