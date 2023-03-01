#include "lnArduino.h"
extern "C"
{

#include "general.h"
#include "ctype.h"
#include "hex_utils.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "gdb_hostio.h"
#include "target.h"

void gdb_target_destroy_callback(target_controller_s *tc, target_s *t)
{
    xAssert(0);
}
 void gdb_target_printf(target_controller_s *tc, const char *fmt, va_list ap)
 {
    xAssert(0);
 }
target_controller_s gdb_controller = {
	.destroy_callback = gdb_target_destroy_callback,
	.printf = gdb_target_printf,

	.open = hostio_open,
	.close = hostio_close,
	.read = hostio_read,
	.write = hostio_write,
	.lseek = hostio_lseek,
	.rename = hostio_rename,
	.unlink = hostio_unlink,
	.stat = hostio_stat,
	.fstat = hostio_fstat,
	.gettimeofday = hostio_gettimeofday,
	.isatty = hostio_isatty,
	.system = hostio_system,
};



target_s *cur_target;
bool shutdown_bmda;

bool bmp_attach(uint32_t target)
{
	    cur_target = target_attach_n(target, &gdb_controller);
		if (cur_target) 
        {
            return true;
        }
        return false;

}
void gdb_if_putchar(char c, int flush)
{
    xAssert(0);
}
char gdb_if_getchar_to(uint32_t timeout)
{
    xAssert(0);
}
void gdb_outf(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	char *buf;
	if (vasprintf(&buf, fmt, ap) < 0)
		return;

	gdb_out(buf);
	free(buf);
	va_end(ap);    
}
int gdb_main_loop(target_controller_s *tc, char *pbuf, size_t pbuf_size, size_t size, bool in_syscall)
{
   xAssert(0);
}
void gdb_putpacket_f(const char *const fmt, ...)
{
    xAssert(0);
}
}