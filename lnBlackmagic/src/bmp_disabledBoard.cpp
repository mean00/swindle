
#include "lnArduino.h"
extern "C"
{

#include "general.h"

#define BOARD_STUB(x) bool x(target *t) {return false;}

//grep undefined /tmp/x | sort | uniq | sed 's/.*reference to .//g' | sed 's/.$/);/g' | sed 's/^/BOARD_STUB(/g' | sed 's/ //g' | uniq

BOARD_STUB(efm32_aap_probe);
BOARD_STUB(efm32_probe);
BOARD_STUB(ke04_probe);
BOARD_STUB(kinetis_mdm_probe);
BOARD_STUB(kinetis_probe);
BOARD_STUB(lpc15xx_probe);
BOARD_STUB(lpc17xx_probe);
BOARD_STUB(lpc43xx_probe);
BOARD_STUB(lpc546xx_probe);
BOARD_STUB(msp432_probe);
BOARD_STUB(nrf51_mdm_probe);
BOARD_STUB(nrf51_probe);
//BOARD_STUB(rp_probe);
//BOARD_STUB(rp_rescue_probe);
BOARD_STUB(sam3x_probe);
BOARD_STUB(sam4l_probe);
BOARD_STUB(samd_probe);
BOARD_STUB(samx5x_probe);
BOARD_STUB(stm32h7_probe);

BOARD_STUB(samx7x_probe);
BOARD_STUB(renesas_probe);

bool cortexa_probe()
{
  return false;
}

}
