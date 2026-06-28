/**
 * @file bmp_disabledBoard.cpp
 * @brief Stub initialisation for disabled/unsupported board targets
 */

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
    /**
     * @brief Return a comma-separated list of enabled board names.
     * @return String literal from LN_BOARDS_ENABLED.
     */
    const char *list_enabled_boards()
    {
        return LN_BOARDS_ENABLED;
    }
    /**
     * @brief GD32VF1 probe stub — always returns false (disabled board).
     * @param target Target descriptor.
     * @return false (not probed).
     */
    bool __xx__gd32vf1_probe(target_s *const target)
    {
        return false;
    }
    /**
     * @brief RISC-V 64-bit probe stub — always returns false (disabled board).
     * @param target Target descriptor.
     * @return false (not probed).
     */
    bool riscv64_probe(target_s *const target)
    {
        return false;
    }
    /**
     * @brief Cortex-A probe stub — always returns false (disabled board).
     * @param apb        ADIv5 access port.
     * @param debug_base Debug register base address.
     * @return false (not probed).
     */
    bool cortexa_probe(adiv5_access_port_s *apb, uint32_t debug_base)
    {
        return false;
    }
}
// EOF
