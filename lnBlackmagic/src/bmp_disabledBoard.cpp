
#include "lnBMPArduino.h"
extern "C"
{

#include "adiv5.h"
#include "general.h"
#define BOARD_STUB(x)                                                                                                  \
    bool x(adiv5_access_port_s *apb)                                                                                   \
    {                                                                                                                  \
        return false;                                                                                                  \
    }

#include "ln_board_stubs.h"
}
extern "C" bool cortexa_probe(adiv5_access_port_s *apb, uint32_t debug_base)
{
    return false;
}

extern "C" const char *list_enabled_boards()
{
    return LN_BOARDS_ENABLED;
}

// stubs riscv stuff

extern "C" bool gd32vf1_probe(target_s *const target)
{
    return false;
}

extern "C" bool ch32v003x_probe(target_s *const target)
{
    return false;
}

extern "C" bool ch32vx_probe(target_s *const target)
{
    return false;
}