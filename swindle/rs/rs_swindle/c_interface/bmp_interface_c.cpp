#include "lnBMPArduino.h"
extern "C"
{

#include "ctype.h"
// #include "gdb_hostio.h"
#include "gdb_if.h"
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "platform.h"
#include "target.h"
#include "target_internal.h"
}
// C++
bool rv_dm_probe(uint32_t *chip_id); // C++
// C
extern "C"
{
    extern "C" bool ch32v3xx_write_user_byte(target_s *target, uint8_t user);
    extern "C" uint8_t ch32v3xx_read_user_byte(target_s *target);
    extern "C" size_t xPortGetFreeHeapSize(void);
    extern "C" size_t xPortGetMinimumEverFreeHeapSize(void);
    extern "C" int command_process(target_s *const t, const char *cmd_buffer);
    extern "C" bool bmd_crc32(target_s *const target, uint32_t *const result, const uint32_t base, const size_t len);
    target_s *cur_target;
    bool shutdown_bmda;

    /*
     *
     */
    const char *bmp_get_driver_name_c()
    {
        if (!cur_target)
            return "";
        return cur_target->driver;
    }

    void gdb_target_destroy_callback(target_controller_s *tc, target_s *t)
    {
        if (t == cur_target)
        {
            cur_target = NULL;
        }
    }
    void gdb_target_printf(target_controller_s *tc, const char *fmt, va_list ap)
    {

        // xAssert(0);
    }
    target_controller_s gdb_controller = {
        .destroy_callback = gdb_target_destroy_callback,
        .printf = gdb_target_printf,
        .semihosting_buffer_ptr = NULL,
    };
    bool bmp_is_riscv_c()
    {
        if (!cur_target)
            return false;
#warning INCORRECT HACK only RV32 supported
        const char *name = cur_target->core;
        if (!name)
            return false;
        if ((name[0] == 'r') && (name[1] == 'v'))
            return true;
        return false;
    }

    /*

    */
    bool bmp_attached_c()
    {
        if (!cur_target)
            return false;

        return cur_target->attached;
    }
    /**
     */
    bool bmp_attach_c(uint32_t target)
    {
        cur_target = target_attach_n(target, &gdb_controller);
        if (cur_target)
        {
            return true;
        }
        return false;
    }
    /**
     */
    bool bmp_detach_c(uint32_t target)
    {
        if (cur_target)
        {
            SET_RUN_STATE(true);
            target_detach(cur_target);
            cur_target = NULL;
        }
        return true;
    }

    static char tmpBuffer[128];
    /**
     */
    void gdb_outf(const char *fmt, ...)
    {
        va_list ap;

        va_start(ap, fmt);
        char *buf;
        // this is suboptimal
        vsnprintf((char *)tmpBuffer, 127, fmt, ap);
        gdb_out(tmpBuffer);
        va_end(ap);
    }

    bool bmp_custom_crc32_c(uint32_t adr, uint32_t size_in_bytes, uint32_t *crc)
    {
        if (!cur_target->crc32)
            return false;
        return cur_target->crc32(cur_target, adr, size_in_bytes, crc);
    }
    int bmp_map_count_c(int kind)
    {
        if (!bmp_attached_c())
        {
            return 0; // DF ?
        }
        int count = 0;
        switch (kind)
        {
        case 0: // flash
        {
            target_flash_s *f = cur_target->flash;
            while (f)
            {
                count++;
                f = f->next;
            }
            return count;
        }
        break;
        case 1: // ram
        {
            target_ram_s *f = cur_target->ram;
            while (f)
            {
                count++;
                f = f->next;
            }
            return count;
        }
        break;
        default:
            xAssert(0);
        }
        return 0;
    }
    /**
     * @brief
     *
     * @return uint32_t
     */
    uint32_t bmp_get_cpuid_c()
    {
        if (!bmp_attached_c())
        {
            return 0; // DF ?
        }
        return cur_target->cpuid;
    }
    /**
     * @brief
     *
     * @param kind
     * @param index
     * @param start
     * @param size
     * @param blockSize
     * @return true
     * @return false
     */
    bool bmp_map_get_c(int kind, int index, uint32_t *start, uint32_t *size, uint32_t *blockSize)
    {
        if (!bmp_attached_c())
        {
            return false; // DF ?
        }
        int count = 0;
        switch (kind)
        {
        case 0: // flash
        {
            target_flash_s *f = cur_target->flash;
            while (f && index != 0)
            {
                count++;
                f = f->next;
                index--;
            }
            if (f)
            {
                *start = f->start;
                *size = f->length;
                *blockSize = f->blocksize;
                return true;
            }
            return false;
        }
        break;
        case 1: // ram
        {
            target_ram_s *f = cur_target->ram;
            while (f && index != 0)
            {
                count++;
                f = f->next;
                index--;
            }
            if (f)
            {
                *start = f->start;
                *size = f->length;
                *blockSize = 0;
                return true;
            }
            return false;
        }
        break;
        default:
            xAssert(0);
        }
        return 0;
    }

    unsigned int bmp_registers_count_c()
    {
        if (!bmp_attached_c())
        {
            return 0;
        }
        return target_regs_size(cur_target) / 4;
    }
    bool bmp_read_register_c(const unsigned int reg, uint32_t *val)
    {
        if (!target_reg_read(cur_target, reg, val, 4))
            return false;
        return true;
    }
    bool bmp_read_all_registers_c(unsigned int *regs)
    {
        if (!bmp_attached_c())
        {
            return false;
        }
        target_regs_read(cur_target, regs);
        return true;
    }

    void *copy = NULL;
    const char *bmp_target_description_c()
    {
        if (!bmp_attached_c())
            return "";
        const char *c = target_regs_description(cur_target);
        if (!c)
            return "";
        copy = (void *)c;
        return c;
    }

    extern "C" void free(void *__ptr)

        ;
    //
    void bmp_target_description_clear_c(const unsigned char *data)
    {
        if (copy)
        {
            free(copy);
            copy = NULL;
        }
    }
    //
    extern "C" bool bmp_ch32v3xx_write_user_option_byte_c(uint8_t memory_conf)
    {
        if (!bmp_attached_c())
            return 0;
        return ch32v3xx_write_user_byte(cur_target, memory_conf);
    }
    //
    extern "C" uint8_t bmp_ch32v3xx_read_user_option_byte_c()
    {
        if (!bmp_attached_c())
            return 0;
        return ch32v3xx_read_user_byte(cur_target);
    }
    //
    bool bmp_write_reg_c(const unsigned int reg, const unsigned int val)
    {
        if (!bmp_attached_c())
            return false;
        if (target_reg_write(cur_target, reg, &val, sizeof(val)) > 0)
            return true;
        return false;
    }
    bool bmp_read_reg_c(const unsigned int reg, unsigned int *val)
    {
        if (!bmp_attached_c())
            return false;
        if (target_reg_read(cur_target, reg, val, sizeof(*val)) > 0)
            return true;
        return false;
    }
    //
    bool bmp_flash_erase_c(const unsigned int addr, const unsigned int length)
    {
        if (!bmp_attached_c())
            return false;
        if (target_flash_erase(cur_target, addr, length))
            return true;
        return false;
    }
    //
    bool bmp_mem_write_c(const unsigned int addr, const unsigned int length, const uint8_t *data)
    {
        if (!bmp_attached_c())
            return false;
        if (target_mem32_write(cur_target, addr, data, length))
            return false;
        return true;
    }
    //
    bool bmp_flash_write_c(const unsigned int addr, const unsigned int length, const uint8_t *data)
    {
        if (!bmp_attached_c())
            return false;
        if (!target_flash_write(cur_target, addr, data, length))
            return false;
#if 0 // Verify			
	if (!target_mem_read(cur_target, tmp, addr, length))
			return false;
	for(int i=0;i<length;i++)
	{
		if( tmp[i]!=data[i])
		{
			printf("Flash write mismatch at %x %x => %x\n",addr+i,data[i],tmp[i]);
		}
	}
#endif
        return true;
    }
    bool bmp_flash_complete_c()
    {
        if (!bmp_attached_c())
            return false;
        if (target_flash_complete(cur_target))
            return true;
        return false;
    }
    bool bmp_mem_read_c(const unsigned int addr, const unsigned int length, uint8_t *data)
    {
        if (!bmp_attached_c())
            return false;
        if (target_mem32_read(cur_target, data, addr, length))
            return false;
        return true;
    }

    bool bmp_reset_target_c()
    {
        if (!bmp_attached_c())
            return false;
        target_reset(cur_target);
        return true;
    }
    bool bmp_add_breakpoint_c(const unsigned int type, const unsigned int address, const unsigned int len)
    {
        if (!bmp_attached_c())
            return false;
        // Error code inverted 0 means success
        if (target_breakwatch_set(cur_target, (target_breakwatch)type, address, len))
            return false;
        return true;
    }
    bool bmp_remove_breakpoint_c(const unsigned int type, const unsigned int address, const unsigned int len)
    {
        if (!bmp_attached_c())
            return false;
        // Error code inverted 0 means success
        if (target_breakwatch_clear(cur_target, (target_breakwatch)type, address, len))
            return false;
        return true;
    }
    bool bmp_target_halt_c()
    {
        if (!bmp_attached_c())
            return false;
        target_halt_request(cur_target);
        return true;
    }
    bool bmp_target_halt_resume_c(bool step)
    {
        if (!bmp_attached_c())
            return false;
        target_halt_resume(cur_target, step);
        return true;
    }
    /**
     *
     */
    unsigned int bmp_poll_target_c(unsigned int *watchpoint)
    {
        if (!bmp_attached_c())
            return 0;

        target_addr64_t watch;
        target_halt_reason_e reason = cur_target->halt_poll(cur_target, &watch);
        *watchpoint = watch; // warning ! 32 bits address only!
        return reason;
    }
    bool bmp_mon_c(const char *str)
    {
        if (command_process(cur_target, str) == 0)
            return true;
        return false;
    }

    /**
     */
    uint32_t free_heap_c()
    {
        return (uint32_t)xPortGetFreeHeapSize();
    }
    /**
     */
    uint32_t min_free_heap_c()
    {
        return (uint32_t)xPortGetMinimumEverFreeHeapSize();
    }
    /*
     *
     */
    extern "C" void usbCdc_Logger(int n, const char *data);
    extern "C" void swindleRedirectLog_c(int32_t toggle)
    {
        Logger("Setting redirect to usb to %d\n", toggle);
        if (toggle)
            setLogger(usbCdc_Logger);
        else
            setLogger(NULL);
        Logger("Setting redirect to usb to %d\n", toggle);
    }
    /*

    z1,addr,kind’ insert hw breakpoint
    z1,addr,kind’ remove hw breakpoint

    kind 2 16-bit Thumb mode breakpoint.
    kind 3 32-bit Thumb mode (Thumb-2) breakpoint.
    kind 4 32-bit ARM mode breakpoint.


    R => run
        target_reset(cur_target);
    vRun -> ignore

    k command
    static void handle_kill_target(void)
    {
        if (cur_target) {
            target_reset(cur_target);
            target_detach(cur_target);
            last_target = cur_target;
            cur_target = NULL;
        }
    }
    */
    /*
    extern bool cmd_swdp_scan(target_s *t, int argc, const char **argv);
    bool cmd_swd_scan(target_s *t, int argc, const char **argv)
    {
        return cmd_swdp_scan(t, argc, argv);
    }
    */
} // extern C

// EOF
