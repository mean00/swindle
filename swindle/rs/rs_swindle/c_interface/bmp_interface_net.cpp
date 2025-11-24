#include "lnBMPArduino.h"
extern "C"
{

#include "ctype.h"
// #include "gdb_hostio.h"
#include "exception.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "platform.h"
#include "rtt_if.h"
#include "target.h"
#include "target_internal.h"
}

extern "C" void swindleRedirectLog_c(int32_t toggle)
{
}
