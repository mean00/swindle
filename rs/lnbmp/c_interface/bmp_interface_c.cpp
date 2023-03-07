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
#include "target_internal.h"


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
/*

*/
bool bmp_attached_c()
{
	if(!cur_target)
		return false;
	
	return target_attached(cur_target);	
}
/**
*/
bool bmp_attach_c(uint32_t target)
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

int bmp_map_count_c(int kind)
{
	if(!bmp_attached_c())
	{
		return 0; // DF ?
	}
	int count=0;
	switch(kind)
	{
		case 0: // flash	
			{
				target_flash_s *f = cur_target->flash;
				while(f)
				{
					count++;
					f=f->next;
				}
				return count;		
			}
			break;
		case 1: // ram	
			{
				target_ram_s *f = cur_target->ram;
				while(f)
				{
					count++;
					f=f->next;
				}
				return count;		
			}
			break;
		default: 
			xAssert(0);
	}
	return 0;
}

bool bmp_map_get_c(int kind, int index, uint32_t *start, uint32_t *size, uint32_t *blockSize)
{
	if(!bmp_attached_c())
	{
		return false; // DF ?
	}
	int count=0;
	switch(kind)
	{
		case 0: // flash	
			{
				target_flash_s *f = cur_target->flash;
				while(f && index!=0)
				{
					count++;
					f=f->next;
					index--;
				}
				if(f)
				{
					*start=f->start;
					*size=f->length;
					*blockSize=f->blocksize;
					return true;
				}
				return false;
			}
			break;
		case 1: // ram	
			{
				target_ram_s *f = cur_target->ram;
				while(f && index!=0)
				{
					count++;
					f=f->next;
					index--;
				}
				if(f)
				{
					*start=f->start;
					*size=f->length;
					*blockSize=0;
					return true;
				}
				return false;
			}
			break;
		default: 
			xAssert(0);
	}
	return 0;
}

unsigned int bmp_registers_count_c()
{
	if( !bmp_attached_c())
	{
		return 0;
	}
	return target_regs_size(cur_target)/4;
}
bool bmp_read_register_c(const unsigned int reg, uint32_t *val)
{
	if(!target_reg_read(cur_target,reg,val,4)) 
        return false;
    return true;
}

const char * bmp_target_description_c()
{
	if(!bmp_attached_c()) return "";
	const char *c=target_regs_description(cur_target);
	if(!c) return "";
	return c;
}
bool bmp_write_reg_c(const unsigned int reg, const unsigned int val)
{
	if(!bmp_attached_c()) return false;
	if (target_reg_write(cur_target, reg, &val, sizeof(val)) > 0)
			return true;
	return false;
}
bool bmp_read_reg_c(const unsigned int reg, unsigned int *val)
{
	if(!bmp_attached_c()) return false;
	if (target_reg_read(cur_target, reg, val, sizeof(*val)) > 0)
			return true;
	return false;
}
bool bmp_flash_erase_c(const unsigned int addr, const unsigned int length)
{
	if(!bmp_attached_c()) return false;
	if (target_flash_erase(cur_target, addr, length))
			return true;
	return false;
}
} // extern C
// EOF

