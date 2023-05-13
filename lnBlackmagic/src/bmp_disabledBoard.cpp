
#include "lnArduino.h"
extern "C"
{

#include "general.h"
#include "adiv5.h"
#define BOARD_STUB(x) bool x(adiv5_access_port_s *apb) {return false;}

#include "ln_board_stubs.h"

bool cortexa_probe(adiv5_access_port_s *apb, uint32_t debug_base)
{
  return false;
}

}

const char *list_enabled_boards()
{
  return LN_BOARDS_ENABLED;
}
