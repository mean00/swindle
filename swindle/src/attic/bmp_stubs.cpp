/*

 */
#include "lnArduino.h"
extern "C"
{
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "morse.h"
#include "target.h"
#include "version.h"

    int gdb_main_loop(target_controller_s *tc, char *pbuf, size_t pbuf_size, size_t size, bool in_syscall)
    {
        xAssert(0);
    }
}