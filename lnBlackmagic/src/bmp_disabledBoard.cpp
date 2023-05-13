
#include "lnArduino.h"
extern "C"
{

#include "general.h"
#include "adiv5.h"
#define BOARD_STUB(x) bool x(adiv5_access_port_s *apb) {return false;}

#include "ln_board_stubs.h"


}

extern "C" const char *list_enabled_boards()
{
  return LN_BOARDS_ENABLED;
}
