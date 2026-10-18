/**
 * @file bmp_helpers.cpp
 * @brief Standalone implementations of small Blackmagic utility functions.
 *
 * Replaces the following files from the blackmagic source tree:
 *   - blackmagic/src/exception.c
 *   - blackmagic/src/maths_utils.c
 *   - blackmagic/src/hex_utils.c
 *   - blackmagic/src/target/stm32_common.c
 *
 * (Note: blackmagic/src/target/gdb_reg.c was completely unused dead code
 *  because Swindle generates all target XML in Rust).
 *
 * This allows removing blackmagic/src/ completely from compilation.
 */

#include <cstdint>
#include <cstddef>
#include <cstdlib>
#include <cstdio>
#include <cstring>
#include <cstdarg>

extern "C"
{
#include "general.h"
#include "exception.h"
#include "maths_utils.h"
#include "hex_utils.h"
#include "target.h"
#include "target_internal.h"
#include "stm32_common.h"
#include "command.h"
#include "gdb_packet.h"
#include "jep106.h"

bool parse_enable_or_disable(const char *value, bool *out);

/* --------------------------------------------------------------------------
 * exception.c replacement
 * -------------------------------------------------------------------------- */
exception_s *innermost_exception = nullptr;

void raise_exception(const uint32_t type, const char *const msg)
{
    (void)type;
    (void)msg;
}

/* --------------------------------------------------------------------------
 * maths_utils.c replacement
 * -------------------------------------------------------------------------- */
uint8_t ulog2(uint32_t value)
{
    if (!value)
        return UINT8_MAX;
    return (uint8_t)(32U - (uint8_t)__builtin_clz(value));
}

uint8_t calculate_odd_parity(const uint32_t value)
{
    return (uint8_t)__builtin_parity(value);
}

/* --------------------------------------------------------------------------
 * hex_utils.c replacement
 * -------------------------------------------------------------------------- */
char hex_digit(const uint8_t value)
{
    return (value > 9U) ? (char)(value - 10U + 'A') : (char)(value + '0');
}

char *hexify(char *const dst, const void *const buf, const size_t size)
{
    const uint8_t *const src = (const uint8_t *)buf;
    size_t dst_idx = 0U;
    for (size_t src_idx = 0U; src_idx < size; ++src_idx)
    {
        dst[dst_idx++] = hex_digit(src[src_idx] >> 4U);
        dst[dst_idx++] = hex_digit(src[src_idx] & 0x0fU);
    }
    return dst;
}

uint8_t unhex_digit(const char hex)
{
    uint8_t tmp = (uint8_t)(hex - '0');
    if (tmp > 9U)
        tmp -= (uint8_t)('A' - '0' - 10U);
    if (tmp > 16U)
        tmp -= (uint8_t)('a' - 'A');
    return tmp;
}

char *unhexify(void *const buf, const char *hex, const size_t size)
{
    uint8_t *const dst = (uint8_t *)buf;
    for (size_t idx = 0; idx < size; ++idx, hex += 2U)
        dst[idx] = (uint8_t)((unhex_digit(hex[0]) << 4U) | unhex_digit(hex[1]));
    return (char *)buf;
}

uint64_t hex_string_to_num(const size_t max_digits, const char *const str)
{
    uint64_t ret = 0;
    for (size_t offset = 0; offset < max_digits; ++offset)
    {
        const char value = str[offset];
        if (!is_hex(value))
            return ret;
        ret <<= 4U;
        ret |= unhex_digit(value);
    }
    return ret;
}

bool read_unum32(const char *input, const char **rest, uint32_t *val, char follow, int base)
{
    char *end = nullptr;
    uint32_t result = (uint32_t)strtoul(input, &end, base);
    if (!end || end == input)
        return false;
    if (follow != READ_HEX_NO_FOLLOW)
    {
        if (*end == follow)
            end++;
        else
            return false;
    }
    if (rest)
        *rest = end;
    if (val)
        *val = result;
    return true;
}



/* --------------------------------------------------------------------------
 * stm32_common.c replacement
 * -------------------------------------------------------------------------- */
bool stm32_uid(target_s *const target, const target_addr_t uid_base)
{
    uint8_t uid_bytes[12] = {0};
    if (target_mem32_read(target, uid_bytes, uid_base, 12))
        return false;
    char uid_hex[25] = {0};
    for (size_t i = 0; i < 12U; i += 4U)
    {
        uint32_t value = (uint32_t)uid_bytes[i] | ((uint32_t)uid_bytes[i + 1] << 8) |
                         ((uint32_t)uid_bytes[i + 2] << 16) | ((uint32_t)uid_bytes[i + 3] << 24);
        snprintf(uid_hex + i * 2U, 9, "%08X", (unsigned int)value);
    }
    tc_printf(target, "0x%s\n", uid_hex);
    return true;
}

/* --------------------------------------------------------------------------
 * target_flash helpers for Rust flash_manager
 * -------------------------------------------------------------------------- */
target_flash_s *target_get_flash(target_s *target)
{
    return target ? target->flash : nullptr;
}

bool target_get_flash_mode(target_s *target)
{
    return target ? target->flash_mode : false;
}

bool target_enter_flash_mode_helper(target_s *target)
{
    if (!target)
        return false;
    if (target->flash_mode)
        return true;

    bool result = true;
    if (target->enter_flash_mode)
        result = target->enter_flash_mode(target);
    else
        target_reset(target);

    if (result)
        target->flash_mode = true;
    return result;
}

bool target_exit_flash_mode_helper(target_s *target)
{
    if (!target)
        return false;
    if (!target->flash_mode)
        return true;

    bool result = true;
    if (target->exit_flash_mode)
        result = target->exit_flash_mode(target);
    else
        target_reset(target);

    target->flash_mode = false;
    return result;
}

bool target_has_mass_erase(target_s *target)
{
    return target && (target->mass_erase != nullptr);
}

bool target_mass_erase_call(target_s *target, platform_timeout_s *timeout)
{
    if (target && target->mass_erase)
        return target->mass_erase(target, timeout);
    return false;
}

/* --------------------------------------------------------------------------
 * target.c replacements and Rust target bridge shims
 * -------------------------------------------------------------------------- */

target_s *target_alloc_c(void)
{
    return (target_s *)calloc(1, sizeof(target_s));
}

void target_free_c(target_s *t)
{
    free(t);
}

void target_detach_safe(target_s *t)
{
    TRY (EXCEPTION_ALL) {
        if (t->attached && t->detach)
            t->detach(t);
    } CATCH () {
    default:
        t->attached = false;
        break;
    }
}

target_ram_s *target_ram_alloc_c(void)
{
    return (target_ram_s *)calloc(1, sizeof(target_ram_s));
}

void target_ram_free_c(target_ram_s *r)
{
    free(r);
}

void target_flash_buf_free_c(void *flash)
{
    target_flash_s *f = (target_flash_s *)flash;
    if (f) {
        if (f->buf)
            free(f->buf);
        free(f);
    }
}

void target_priv_free_c(target_s *t)
{
    if (t && t->priv && t->priv_free)
        t->priv_free(t->priv);
}

void target_storage_free_c(target_s *t)
{
    if (t && t->target_storage)
        free(t->target_storage);
}

void target_commands_free_c(target_s *t)
{
    if (!t)
        return;
    while (t->commands) {
        target_command_s *const tc = t->commands->next;
        free(t->commands);
        t->commands = tc;
    }
}

void target_bw_list_free_c(target_s *t)
{
    if (!t)
        return;
    while (t->bw_list) {
        void *next = t->bw_list->next;
        free(t->bw_list);
        t->bw_list = (breakwatch_s *)next;
    }
}

void target_get_tc_destroy_c(target_s *t)
{
    if (t && t->tc && t->tc->destroy_callback)
        t->tc->destroy_callback(t->tc, t);
}

target_s *target_get_next_c(target_s *t)
{
    return t ? t->next : nullptr;
}

void target_set_next_c(target_s *t, target_s *next)
{
    if (t)
        t->next = next;
}

bool target_get_attached_c(target_s *t)
{
    return t ? t->attached : false;
}

void target_set_attached_c(target_s *t, bool att)
{
    if (t)
        t->attached = att;
}

void target_set_tc_c(target_s *t, void *tc)
{
    if (t)
        t->tc = (target_controller_s *)tc;
}

bool target_call_attach_c(target_s *t)
{
    if (!t)
        return false;
    return t->attach ? t->attach(t) : true;
}

void target_call_detach_c(target_s *t)
{
    if (t && t->detach)
        t->detach(t);
}

bool target_call_check_error_c(target_s *t)
{
    if (t && t->check_error)
        return t->check_error(t);
    return false;
}

void target_call_mem_read_c(target_s *t, void *dest, uint64_t src, size_t len)
{
    if (t && t->mem_read)
        t->mem_read(t, dest, src, len);
}

void target_call_mem_write_c(target_s *t, uint64_t dest, const void *src, size_t len)
{
    if (t && t->mem_write)
        t->mem_write(t, dest, src, len);
}

bool target_call_mem_read_nostop_c(target_s *t, void *dest, uint64_t src, size_t len)
{
    if (!t)
        return false;
    if (t->mem_read_nostop)
        return t->mem_read_nostop(t, dest, src, len);
    if (t->target_options & TOPT_NON_HALTING_MEM_IO) {
        if (t->mem_read)
            t->mem_read(t, dest, src, len);
        return true;
    }
    return false;
}

bool target_call_mem_write_nostop_c(target_s *t, uint64_t dest, const void *src, size_t len)
{
    if (!t)
        return false;
    if (t->mem_write_nostop)
        return t->mem_write_nostop(t, dest, src, len);
    if (t->target_options & TOPT_NON_HALTING_MEM_IO) {
        if (t->mem_write)
            t->mem_write(t, dest, src, len);
        return true;
    }
    return false;
}

bool target_needs_halt_for_mem_c(target_s *t)
{
    if (!t)
        return true;
    return !(t->target_options & TOPT_NON_HALTING_MEM_IO);
}

size_t target_call_reg_read_c(target_s *t, uint32_t reg, void *data, size_t max)
{
    if (t && t->reg_read)
        return t->reg_read(t, reg, data, max);
    return 0;
}

size_t target_call_reg_write_c(target_s *t, uint32_t reg, const void *data, size_t size)
{
    if (t && t->reg_write)
        return t->reg_write(t, reg, data, size);
    return 0;
}

bool target_call_regs_read_c(target_s *t, void *data)
{
    if (t && t->regs_read) {
        t->regs_read(t, data);
        return true;
    }
    return false;
}

bool target_call_regs_write_c(target_s *t, const void *data)
{
    if (t && t->regs_write) {
        t->regs_write(t, data);
        return true;
    }
    return false;
}

size_t target_get_regs_size_c(target_s *t)
{
    return t ? t->regs_size : 0;
}

void target_call_reset_c(target_s *t)
{
    if (t && t->reset)
        t->reset(t);
}

void target_call_halt_request_c(target_s *t)
{
    if (t && t->halt_request)
        t->halt_request(t);
}

uint32_t target_call_halt_poll_c(target_s *t, uint64_t *watch)
{
    if (t && t->halt_poll)
        return (uint32_t)t->halt_poll(t, (target_addr64_t *)watch);
    return TARGET_HALT_RUNNING;
}

void target_call_halt_resume_c(target_s *t, bool step)
{
    if (t && t->halt_resume)
        t->halt_resume(t, step);
}

target_ram_s *target_get_ram_c(target_s *t)
{
    return t ? t->ram : nullptr;
}

void target_set_ram_c(target_s *t, target_ram_s *ram)
{
    if (t)
        t->ram = ram;
}

void *target_get_flash_c(target_s *t)
{
    return t ? (void *)t->flash : nullptr;
}

void target_set_flash_c(target_s *t, void *flash)
{
    if (t)
        t->flash = (target_flash_s *)flash;
}

void *target_flash_get_next_c(void *flash)
{
    return flash ? (void *)(((target_flash_s *)flash)->next) : nullptr;
}

void target_flash_setup_c(target_s *t, void *flash)
{
    if (!t || !flash)
        return;
    target_flash_s *f = (target_flash_s *)flash;
    if (f->writesize == 0)
        f->writesize = f->blocksize;
    if (f->writesize <= 1024U / 2U) {
        const size_t count = 1024U / f->writesize;
        f->writebufsize = f->writesize * count;
    } else {
        f->writebufsize = f->writesize;
    }
    f->t = t;
    f->next = t->flash;
    t->flash = f;
}

/* --------------------------------------------------------------------------
 * Commands and diagnostic logging (formerly in target.c)
 * -------------------------------------------------------------------------- */

void tc_printf(target_s *target, const char *fmt, ...)
{
    if (!target || !target->tc || !target->tc->printf)
        return;
    va_list ap;
    va_start(ap, fmt);
    target->tc->printf(target->tc, fmt, ap);
    va_end(ap);
}

void target_print_progress(platform_timeout_s *const timeout)
{
    if (platform_timeout_is_expired(timeout)) {
        gdb_out(".");
        platform_timeout_set(timeout, 500);
    }
}

static bool target_cmd_mass_erase(target_s *const target, const int argc, const char **const argv)
{
    (void)argc;
    (void)argv;
    if (!target) {
        gdb_out("Mass erase not available\n");
        return true;
    }
    gdb_out("Erasing device Flash: ");
    const bool result = target_flash_mass_erase(target);
    gdb_out("done\n");
    return result;
}

static bool target_cmd_range_erase(target_s *const target, const int argc, const char **const argv)
{
    if (argc < 3) {
        gdb_out("usage: monitor erase_range <address> <count>\n");
        gdb_out("\t<address> is an address in the first page to erase\n");
        gdb_out("\t<count> is the number bytes after that to erase, rounded to the next higher whole page\n");
        return true;
    }
    const uint32_t addr = strtoul(argv[1], NULL, 0);
    const uint32_t length = strtoul(argv[2], NULL, 0);

    return target_flash_erase(target, addr, length);
}

static bool target_cmd_redirect_output(target_s *target, int argc, const char **argv)
{
    if (argc == 1) {
        gdb_outf("Semihosting stdout redirection: %s\n", target->stdout_redirected ? "enabled" : "disabled");
        return true;
    }
    return parse_enable_or_disable(argv[1], &target->stdout_redirected);
}

const command_s target_cmd_list[] = {
    {"erase_mass", target_cmd_mass_erase, "Erase whole device Flash"},
    {"erase_range", target_cmd_range_erase, "Erase a range of memory on a device"},
    {"redirect_stdout", target_cmd_redirect_output, "Redirect semihosting output to aux USB serial"},
    {NULL, NULL, NULL},
};

void target_add_commands(target_s *target, const command_s *cmds, const char *name)
{
    target_command_s *command = (target_command_s *)malloc(sizeof(*command));
    if (!command)
        return;
    if (target->commands) {
        target_command_s *tail;
        for (tail = target->commands; tail->next; tail = tail->next)
            continue;
        tail->next = command;
    } else
        target->commands = command;

    command->specific_name = name;
    command->cmds = cmds;
    command->next = NULL;
}

void target_add_default_commands_c(target_s *t)
{
    target_add_commands(t, target_cmd_list, "Target");
}

void target_command_help(target_s *target)
{
    for (const target_command_s *target_commands = target->commands; target_commands;
         target_commands = target_commands->next) {
        tc_printf(target, "%s specific commands:\n", target_commands->specific_name);
        for (const command_s *command = target_commands->cmds; command->cmd; command++)
            tc_printf(target, "\t%s -- %s\n", command->cmd, command->help);
    }
}

int target_command(target_s *target, int argc, const char *argv[])
{
    for (const target_command_s *target_commands = target->commands; target_commands;
         target_commands = target_commands->next) {
        for (const command_s *command = target_commands->cmds; command->cmd; command++) {
            if (!strncmp(argv[0], command->cmd, strlen(argv[0])))
                return command->handler(target, argc, argv) ? 0 : 1;
        }
    }
    return -1;
}

/* --------------------------------------------------------------------------
 * Breakpoints and helpers (formerly in target.c)
 * -------------------------------------------------------------------------- */

int target_breakwatch_set(target_s *target, target_breakwatch_e type, target_addr_t addr, size_t len)
{
    breakwatch_s bw;
    memset(&bw, 0, sizeof(bw));
    bw.type = type;
    bw.addr = addr;
    bw.size = len;
    int ret = 1;

    if (target->breakwatch_set)
        ret = target->breakwatch_set(target, &bw);

    if (ret == 0) {
        breakwatch_s *bwm = (breakwatch_s *)malloc(sizeof(bw));
        if (!bwm)
            return 1;
        memcpy(bwm, &bw, sizeof(bw));
        bwm->next = target->bw_list;
        target->bw_list = bwm;
    }

    return ret;
}

int target_breakwatch_clear(target_s *target, target_breakwatch_e type, target_addr_t addr, size_t len)
{
    breakwatch_s *bwp = NULL;
    breakwatch_s *bw;
    for (bw = target->bw_list; bw; bwp = bw, bw = bw->next) {
        if (bw->type == type && bw->addr == addr && bw->size == len)
            break;
    }

    if (bw == NULL)
        return -1;

    int ret = 1;
    if (target->breakwatch_clear)
        ret = target->breakwatch_clear(target, bw);

    if (ret == 0) {
        if (bwp == NULL)
            target->bw_list = bw->next;
        else
            bwp->next = bw->next;
        free(bw);
    }
    return ret;
}

bool breakpoint_available(target_s *target)
{
    return !target->no_hw_breakpoint;
}

void target_breakpoint_watchpoint_count(target_s *const target, uint32_t *brk, uint32_t *wtch)
{
    if (target->breakpoint_watchpoint_count) {
        target->breakpoint_watchpoint_count(target, brk, wtch);
        return;
    }
    *brk = 0;
    *wtch = 0;
}

bool target_has_hw_breakpoint(target_s *const target)
{
    return !target->no_hw_breakpoint;
}

const sw_breakpoint_helpers *target_get_mw_helpers(target_s *const target)
{
    return target->sw_breakpoint_helpers;
}

bool target_enter_flash_mode_stub(target_s *target)
{
    (void)target;
    return true;
}

const char *target_get_driver_name_c(target_s *t)
{
    return t ? t->driver : "";
}

uint32_t target_get_cpuid_c(target_s *t)
{
    return t ? t->cpuid : 0;
}

uint32_t target_get_options_c(target_s *t)
{
    return t ? t->target_options : 0;
}

const char *target_get_core_c(target_s *t)
{
    return t ? t->core : "";
}

bool target_has_mw_helpers_c(target_s *t)
{
    return (t && t->sw_breakpoint_helpers) ? true : false;
}

uint32_t target_mw_page_size_c(target_s *t)
{
    if (!t || !t->sw_breakpoint_helpers)
        return 0;
    return t->sw_breakpoint_helpers->page_size(t);
}

bool target_overwrite_flash_c(target_s *t, uint32_t address, const uint8_t *data, uint32_t len)
{
    if (!t || !t->sw_breakpoint_helpers)
        return false;
    const sw_breakpoint_helpers *helpers = t->sw_breakpoint_helpers;
    uint32_t page_size = helpers->page_size(t);
    if (page_size != len)
        return false;
    if (!helpers->page_erase(t, address))
        return false;
    return helpers->page_write(t, address, data, len);
}

void *target_get_priv_c(target_s *t)
{
    return t ? t->priv : nullptr;
}

uint16_t target_get_designer_code_c(target_s *t)
{
    return t ? t->designer_code : 0;
}

struct target_riscv_ops_s {
    void *priv;
    priv_free_func priv_free;
    const char *driver;
    const char *core;
    uint16_t designer_code;
    size_t regs_size;
    bool (*attach)(target_s *);
    void (*detach)(target_s *);
    bool (*check_error)(target_s *);
    void (*mem_read)(target_s *, void *, target_addr64_t, size_t);
    void (*mem_write)(target_s *, target_addr64_t, const void *, size_t);
    bool (*mem_read_nostop)(target_s *, void *, target_addr64_t, size_t);
    bool (*mem_write_nostop)(target_s *, target_addr64_t, const void *, size_t);
    size_t (*reg_read)(target_s *, uint32_t, void *, size_t);
    size_t (*reg_write)(target_s *, uint32_t, const void *, size_t);
    void (*regs_read)(target_s *, void *);
    void (*regs_write)(target_s *, const void *);
    void (*reset)(target_s *);
    void (*halt_request)(target_s *);
    void (*halt_resume)(target_s *, bool);
    target_halt_reason_e (*halt_poll)(target_s *, target_addr64_t *);
    int (*breakwatch_set)(target_s *, breakwatch_s *);
    int (*breakwatch_clear)(target_s *, breakwatch_s *);
};

void target_set_riscv_ops_c(target_s *t, const target_riscv_ops_s *ops)
{
    if (!t || !ops) return;
    t->priv = ops->priv;
    t->priv_free = ops->priv_free;
    t->driver = ops->driver;
    t->core = ops->core;
    t->designer_code = ops->designer_code;
    t->regs_size = ops->regs_size;
    t->attach = ops->attach;
    t->detach = ops->detach;
    t->check_error = ops->check_error;
    t->mem_read = ops->mem_read;
    t->mem_write = ops->mem_write;
    t->mem_read_nostop = ops->mem_read_nostop;
    t->mem_write_nostop = ops->mem_write_nostop;
    t->reg_read = ops->reg_read;
    t->reg_write = ops->reg_write;
    t->regs_read = ops->regs_read;
    t->regs_write = ops->regs_write;
    t->reset = ops->reset;
    t->halt_request = ops->halt_request;
    t->halt_resume = ops->halt_resume;
    t->halt_poll = ops->halt_poll;
    t->breakwatch_set = ops->breakwatch_set;
    t->breakwatch_clear = ops->breakwatch_clear;
    TOPT_SET_ARCH(t->target_options, TOPT_ARCH_RISCV);
}

extern "C" bool ch32v003x_probe(target_s *target);
extern "C" bool ch32v3xx_probe(target_s *target);
extern "C" bool rp2350_probe(target_s *target);

bool riscv32_silicon_probe_c(target_s *target)
{
    if (!target) return false;
    if (target->designer_code == JEP106_MANUFACTURER_WCH) {
        if (ch32v003x_probe(target)) return true;
        if (ch32v3xx_probe(target)) return true;
    } else if (target->designer_code == JEP106_MANUFACTURER_RASPBERRY) {
        if (rp2350_probe(target)) return true;
    }
    if (ch32v003x_probe(target)) return true;
    if (ch32v3xx_probe(target)) return true;
    if (rp2350_probe(target)) return true;
    return false;
}

} // extern "C"
