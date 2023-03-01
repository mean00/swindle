#include "lnArduino.h"
extern "C"
{

#include "general.h"
#include "ctype.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "target.h"

    void gdb_if_putchar(char c, int flush)
    {
        xAssert(0);
    }
    char gdb_if_getchar_to(uint32_t timeout)
    {
        xAssert(0);
    }
    int gdb_main_loop(target_controller_s *tc, char *pbuf, size_t pbuf_size, size_t size, bool in_syscall)
    {
    xAssert(0);
    }
    void gdb_putpacket_f(const char *const fmt, ...)
    {
        xAssert(0);
    }

    int vasprintf_(char **strp, const char *fmt, va_list ap)
    {
        xAssert(0);
    }
}
// EOF