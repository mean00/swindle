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
/**
*/
bool bmp_attach(uint32_t target)
{
	    cur_target = target_attach_n(target, &gdb_controller);
		if (cur_target) 
        {
            return true;
        }
        return false;

}
static char tmpBuffer[128];
/**
*/
void gdb_outf(const char *fmt, ...)
{    
	va_list ap;

	va_start(ap, fmt);
	char *buf;
    // this is suboptimal
    vsnprintf((char *)tmpBuffer,127,fmt,ap);    
	gdb_out(tmpBuffer);
	va_end(ap);    
}
}