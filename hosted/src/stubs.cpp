#include "stdlib.h"
#include "stdio.h"
#include "string.h"
extern "C" void do_assert(const char *z)
{
    printf("FATAL ERROR :%s\n",z);
    exit(-1);

}
extern "C" void _putchar(char) {};
extern "C" void bmp_set_wait_state_c(unsigned int ws) {}
