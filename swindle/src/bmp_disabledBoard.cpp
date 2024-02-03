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
 * @brief 
 * 
 * @return const char* 
 */
const char *list_enabled_boards()
{
    return LN_BOARDS_ENABLED;
}
/**
 * @brief 
 * 
 * @param target 
 * @return true 
 * @return false 
 */
bool gd32vf1_probe(target_s *const target)
{
    return false;
}
/**
 * @brief 
 * 
 * @param target 
 * @return true 
 * @return false 
 */
bool riscv64_probe(target_s *const target)
{
    return false;
}
/**
 * @brief 
 * 
 * @param apb 
 * @param debug_base 
 * @return true 
 * @return false 
 */
bool cortexa_probe(adiv5_access_port_s *apb, uint32_t debug_base)
{
    return false;
}

}
// EOF
