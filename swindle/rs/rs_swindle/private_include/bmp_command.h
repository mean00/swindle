
#include "stdint.h"
typedef void target_s;
typedef int bool;

bool bmp_custom_crc32_c(uint32_t adr, uint32_t size_in_bytes, uint32_t *crc);
const unsigned char *bmp_get_driver_name_c();
bool bmp_ch32v3xx_write_user_option_byte_c(uint8_t memory_conf);
uint8_t bmp_ch32v3xx_read_user_option_byte_c();

void bmp_set_wait_state_c(unsigned int ws); // this is used to set the clock on SWD
unsigned int bmp_get_wait_state_c();        // this is used to set the clock on SWD
//
void bmp_set_frequency_c(unsigned int fs); // this is used to set the clock on SWD
unsigned int bmp_get_frequency_c();        // this is used to set the clock on SWD

bool cmd_swd_scan(const target_s *t, int argc, const unsigned char **argv);
bool cmd_rvswd_scan(const target_s *t, int argc, const unsigned char **argv);

bool bmp_attach_c(uint32_t target);
bool bmp_detach_c(uint32_t target);
bool bmp_attached_c();
bool bmp_is_riscv_c();

int bmp_map_count_c(const uint32_t kind);
bool bmp_map_get_c(const uint32_t, const uint32_t, uint32_t *start, uint32_t *size, uint32_t *blockSize);

unsigned int bmp_registers_count_c();
bool bmp_read_register_c(const unsigned int reg, unsigned int *val);
bool bmp_read_registers_c(unsigned int *val); // carefull, must be able to store all regs
bool bmp_read_all_registers_c(unsigned int *regs);
bool bmp_write_all_registers_c(const unsigned int *regs);
const unsigned char *bmp_target_description_c();
void bmp_target_description_clear_c();

bool bmp_write_reg_c(const unsigned int reg, const unsigned int value);
bool bmp_read_reg_c(const unsigned int reg, unsigned int *value);
uint32_t bmp_get_cpuid_c();
bool bmp_flash_erase_c(const unsigned int addr, const unsigned int length);
bool bmp_flash_write_c(const unsigned int addr, const unsigned int length, const uint8_t *data);
bool bmp_flash_complete_c();
float bmp_get_target_voltage_c();

bool bmp_mem_read_c(const unsigned int addr, const unsigned int length, uint8_t *data);

bool bmp_crc32_c(const unsigned int address, unsigned int length, unsigned int *crc);

bool bmp_reset_target_c();
void rv_dm_start_c();
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
void swindleRedirectLog_c(bool onoff);

// platform
void platform_nrst_set_val(bool assert);
bool platform_nrst_get_val();
const unsigned char *platform_target_voltage(void);
void platform_target_clk_output_enable(bool enable);

//
void Logger2(int n, const unsigned char *fmt);
const unsigned char *list_enabled_boards();
//
void bmp_pin_set(uint8_t pin, uint8_t value);
uint8_t bmp_pin_get(uint8_t pin);
void bmp_pin_direction(uint8_t pin, uint8_t output); // output = 1, input =0
//
void bmp_test();
const unsigned char *bmp_get_version_string(void);
//
bool bmp_mon_c(const unsigned char *str);
uint32_t free_heap_c();
uint32_t min_free_heap_c();

//
bool bmp_rv_dm_read_c(uint8_t adr, uint32_t *value);
bool bmp_rv_dm_write_c(uint8_t adr, uint32_t value);
bool bmp_rv_dm_reset_c();

bool bmp_rv_rvswd_probe_c(uint32_t *id);

int platform_buffer_read(uint8_t *data, int maxsize);
int platform_buffer_write(const uint8_t *data, int size);
int platform_buffer_write_buffered(const uint8_t *data, int size);
void platform_write_flush();

// adiv
void bmp_clear_dp_fault_c();

bool bmp_adiv5_swd_write_no_check_c(const uint16_t addr, const uint32_t data);
uint32_t bmp_adiv5_swd_read_no_check_c(const uint16_t addr);
void bmp_raw_swd_write_c(uint32_t tick, uint32_t value);
uint32_t bmp_adiv5_swd_raw_access_c(const uint8_t rnw, const uint16_t addr, const uint32_t value, uint32_t *fault);
#include "bmp_rtt_command.h"
void bmp_rtt_get_info_c(rttInfo *info);
void bmp_rtt_set_info_c(rttField field, const rttInfo *info);
//
void bmp_raise_exception_c();
bool bmp_try_c();
int bmp_catch_c();
void bmp_enable_reset_pin_c(bool enabled);
// EOF
