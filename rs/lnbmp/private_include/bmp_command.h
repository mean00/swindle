
#include "stdint.h"
typedef void target_s ;
typedef int bool ;


bool cmd_swdp_scan(const target_s *t, int argc, const char **argv);

bool bmp_attach_c(uint32_t target);
bool bmp_detach_c(uint32_t target);
bool bmp_attached_c();

int  bmp_map_count_c(const uint32_t  kind);
bool bmp_map_get_c(const uint32_t, const uint32_t, uint32_t *start, uint32_t *size, uint32_t *blockSize);


unsigned int bmp_registers_count_c();
bool         bmp_read_register_c(const unsigned int reg, unsigned int *val);
const char * bmp_target_description_c();
void bmp_target_description_clear_c( );

bool bmp_write_reg_c(const unsigned int reg, const unsigned int value);
bool bmp_read_reg_c (const unsigned int reg, unsigned int *value);

bool bmp_flash_erase_c(const unsigned int addr, const unsigned int length);
bool bmp_flash_write_c(const unsigned int addr, const unsigned int length, const uint8_t *data);
bool bmp_flash_complete_c();
float bmp_get_target_voltage_c();

bool bmp_mem_read_c(const unsigned int addr, const unsigned int length, uint8_t *data);

bool bmp_crc32_c(const unsigned int address, unsigned int length, unsigned int *crc);

bool bmp_reset_target_c();
bool bmp_add_breakpoint_c(const unsigned int type, const unsigned int address, const unsigned int len);
bool bmp_remove_breakpoint_c(const unsigned int type, const unsigned int address, const unsigned int len);

bool bmp_target_halt_resume_c(bool step);
bool bmp_target_halt_c();

unsigned int bmp_poll_target_c(unsigned int *watchpoint);

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
