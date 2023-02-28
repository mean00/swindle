
#include "stdint.h"
typedef void target_s ;
typedef int bool ;
bool cmd_swdp_scan(const target_s *t, int argc, const char **argv);
bool bmp_attach(uint32_t target);
/*
bool cmd_auto_scan(target_s *t, int argc, const char **argv);
bool cmd_frequency(target_s *t, int argc, const char **argv);
bool cmd_targets(target_s *t, int argc, const char **argv);
bool cmd_morse(target_s *t, int argc, const char **argv);
bool cmd_halt_timeout(target_s *t, int argc, const char **argv);
bool cmd_connect_reset(target_s *t, int argc, const char **argv);
bool cmd_reset(target_s *t, int argc, const char **argv);
bool cmd_tdi_low_reset(target_s *t, int argc, const char **argv);
*/
