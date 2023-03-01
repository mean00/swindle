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
    

    int vasprintf_(char **strp, const char *fmt, va_list ap)
    {
        xAssert(0);
    }

    void gdb_putpacket_f(const char *const fmt, ...)
    {
        xAssert(0);
    }

int hostio_reply(target_controller_s *tc, char *pbuf, int len)
{
	 xAssert(0);
}

/* Interface to host system calls */
int hostio_open(target_controller_s *tc, target_addr_t path, size_t path_len, target_open_flags_e flags, mode_t mode)
{
	 xAssert(0);
}

int hostio_close(target_controller_s *tc, int fd)
{
	 xAssert(0);
}

int hostio_read(target_controller_s *tc, int fd, target_addr_t buf, unsigned int count)
{
	 xAssert(0);
}

int hostio_write(target_controller_s *tc, int fd, target_addr_t buf, unsigned int count)
{
	 xAssert(0);
}

long hostio_lseek(target_controller_s *tc, int fd, long offset, target_seek_flag_e flag)
{
	 xAssert(0);
}

int hostio_rename(target_controller_s *tc, target_addr_t oldpath, size_t old_len, target_addr_t newpath, size_t new_len)
{
	 xAssert(0);
}

int hostio_unlink(target_controller_s *tc, target_addr_t path, size_t path_len)
{
	 xAssert(0);
}

int hostio_stat(target_controller_s *tc, target_addr_t path, size_t path_len, target_addr_t buf)
{
	 xAssert(0);
}

int hostio_fstat(target_controller_s *tc, int fd, target_addr_t buf)
{
	 xAssert(0);
}

int hostio_gettimeofday(target_controller_s *tc, target_addr_t tv, target_addr_t tz)
{
	 xAssert(0);
}

int hostio_isatty(target_controller_s *tc, int fd)
{
	 xAssert(0);
}

int hostio_system(target_controller_s *tc, target_addr_t cmd, size_t cmd_len)
{
	 xAssert(0);
}

}
// EOF