#include "stdio.h"
#include "stdint.h"
#include "stdarg.h"
#include "stdarg.h"
#include "string.h"

#include <unistd.h>
#include <time.h>

#define HOSTED_BMP_ONLY 1
extern "C"
{
    #include "../hosted/cli.h"
    #include "../hosted/bmp_hosted.h"
}

#include <QSerialPort>
#include <QDebug>


#define xAssert(x) if(!(x)) {printf("%s FAILED in %s \n",#x,__PRETTY_FUNCTION__);exit(0);}

extern "C" int platform_buffer_read(uint8_t *data, int maxsize)
{
    xAssert(0);
    return 0;
}

extern "C" int platform_buffer_write(const uint8_t *data, int size)
{
    xAssert(0);
    return 0;
}

extern "C"  int find_debuggers(bmda_cli_options_s *cl_opts, bmp_info_s *info)
{
    xAssert(0);    
    return 0;
}

extern "C" void libusb_exit_function(bmp_info_s *info)
{
    xAssert(0);       
}

extern "C" void bmp_ident(bmp_info_s *info)
{
    xAssert(0);       
}

extern "C" int serial_open(const bmda_cli_options_s *opt, const char *serial)
{
    xAssert(0);    
    return 0;
}

// EOF