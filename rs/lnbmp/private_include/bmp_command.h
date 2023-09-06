
#include "stdint.h"
typedef void target_s;
typedef int bool;

void bmp_set_wait_state_c(unsigned int ws); // this is used to set the clock on SWD

bool cmd_swd_scan(const target_s *t, int argc, const char **argv);
bool cmd_rvswd_scan(const target_s *t, int argc, const char **argv);

bool bmp_attach_c(uint32_t target);
bool bmp_detach_c(uint32_t target);
bool bmp_attached_c();

int bmp_map_count_c(const uint32_t kind);
bool bmp_map_get_c(const uint32_t, const uint32_t, uint32_t *start, uint32_t *size, uint32_t *blockSize);

unsigned int bmp_registers_count_c();
bool bmp_read_register_c(const unsigned int reg, unsigned int *val);
bool bmp_read_registers_c(unsigned int *val); // carefull, must be able to store all regs
const char *bmp_target_description_c();
void bmp_target_description_clear_c();

bool bmp_write_reg_c(const unsigned int reg, const unsigned int value);
bool bmp_read_reg_c(const unsigned int reg, unsigned int *value);

bool bmp_flash_erase_c(const unsigned int addr, const unsigned int length);
bool bmp_flash_write_c(const unsigned int addr, const unsigned int length, const uint8_t *data);
bool bmp_flash_complete_c();
float bmp_get_target_voltage_c();

bool bmp_mem_read_c(const unsigned int addr, const unsigned int length, uint8_t *data);

bool bmp_crc32_c(const unsigned int address, unsigned int length, unsigned int *crc);

bool bmp_reset_target_c();
bool bmp_add_breakpoint_c(const unsigned int type, const unsigned int address, const unsigned int len);
bool bmp_remove_breakpoint_c(const unsigned int type, const unsigned int address, const unsigned int len);
uint32_t riscv_list_csr(uint32_t start, uint32_t max_size, uint32_t *csr);
bool bmp_target_halt_resume_c(bool step);
bool bmp_target_halt_c();

unsigned int bmp_poll_target_c(unsigned int *watchpoint);

bool bmp_rpc_init_swd_c();
bool bmp_rpc_swd_in_c(uint32_t *value, const unsigned int nb_bits);
bool bmp_rpc_swd_in_par_c(uint32_t *value, bool *par, const unsigned int nb_bits);
bool bmp_rpc_swd_out_c(const uint32_t value, const unsigned int nb_bits);
bool bmp_rpc_swd_out_par_c(const uint32_t value, const unsigned int nb_bits);
//
bool bmp_mem_read_c(uint32_t address, uint32_t len, uint8_t *data);
bool bmp_mem_write_c(uint32_t address, uint32_t len, const uint8_t *data);
//
bool bmp_adiv5_full_dp_read_c(const uint32_t device_index, const uint32_t ap_selection, const uint16_t address,
                              int32_t *err, uint32_t *value);
bool bmp_adiv5_full_dp_low_level_c(const uint32_t device_index, const uint32_t ap_selection, const uint16_t address,
                                   const uint32_t value, int32_t *err, uint32_t *outvalue);

uint32_t bmp_adiv5_ap_read_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t address);
void bmp_adiv5_ap_write_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t address,
                          uint32_t value);

int32_t bmp_adiv5_mem_read_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t csw,
                             const uint32_t address, uint8_t *buffer, uint32_t len);

int32_t bmp_adiv5_mem_write_c(const uint32_t device_index, const uint32_t ap_selection, const uint32_t csw,
                              const uint32_t address, const uint32_t align, const uint8_t *buffer, uint32_t len);

// platform
void platform_nrst_set_val(bool assert);
bool platform_nrst_get_val();
const char *platform_target_voltage(void);
void platform_target_clk_output_enable(bool enable);

//
void Logger2(int n, const char *fmt);
const char *list_enabled_boards();
//
void bmp_pin_set(uint8_t pin, uint8_t value);
uint8_t bmp_pin_get(uint8_t pin);
void bmp_pin_direction(uint8_t pin, uint8_t output); // output = 1, input =0
//
void bmp_test();
const char *bmp_get_version_string(void);
//
bool bmp_mon_c(const unsigned char *str);
uint32_t free_heap_c();
uint32_t min_free_heap_c();
