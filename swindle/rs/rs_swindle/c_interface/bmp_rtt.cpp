
#include "lnBMPArduino.h"
extern "C"
{
#include "rtt_if.h"
#include "../private_include/bmp_rtt_command.h"
    extern bool rtt_enabled;           // rtt on/off
    extern bool rtt_found;             // control block found
    extern uint32_t rtt_cbaddr;        // control block address
    extern uint32_t rtt_min_poll_ms;   // min time between polls (ms)
    extern uint32_t rtt_max_poll_ms;   // max time between polls (ms)
    extern uint32_t rtt_poll_ms;       // current amount of time between polls (ms)
    extern uint32_t rtt_max_poll_errs; // max number of errors before disconnect
    extern bool rtt_flag_ram;          // limit ram scanned by rtt to range rtt_ram_start .. rtt_ram_end
    extern uint32_t rtt_ram_start;     // if rtt_flag_ram set, lower limit of ram scanned by rtt
    extern uint32_t rtt_ram_end;       // if rtt_flag_ram set, upper limit of ram scanned by rtt
    extern uint32_t rtt_cbaddr;
    extern uint32_t rtt_num_up_chan;
    extern uint32_t rtt_num_down_chan;
    /**
     *
     *
     */
    void bmp_rtt_get_info_c(rttInfo *info)
    {
        info->enabled = rtt_enabled;
        info->min_address = rtt_ram_start;
        info->max_address = rtt_ram_end;
        info->min_poll_ms = rtt_min_poll_ms;
        info->max_poll_ms = rtt_max_poll_ms;
        info->max_poll_error = rtt_max_poll_errs;
        //
        info->cb_address = rtt_cbaddr;
        info->found = rtt_found;
    }
    /**
     *
     *
     */
    void bmp_rtt_set_info_c(rttField field, const rttInfo *info)
    {
        switch (field)
        {
        case ENABLED:
            rtt_enabled = info->enabled;
            break;
        case ADDRESS:
            rtt_ram_start = info->min_address;
            rtt_ram_end = info->max_address;
            if (rtt_ram_start || rtt_ram_start)
            {
                rtt_flag_ram = true;
            }
            break;
        case POLLING:
            rtt_min_poll_ms = info->min_poll_ms;
            rtt_max_poll_ms = info->max_poll_ms;
            rtt_max_poll_errs = info->max_poll_error;
            break;
        default:
            xAssert(0);
            break;
        }
    }

    //---------
    /**
     *
     *
     */
    int rtt_if_init(void)
    {
        return 0;
    }
    /* hosted teardown */
    /**
     *
     *
     */
    int rtt_if_exit(void)
    {
        return 0;
    }

    /* target to host: write len bytes from the buffer on the channel starting at buf. return number bytes written */
    /**
     *
     *
     */
    extern "C" void usbCdc_Logger(int n, const char *data);

    uint32_t rtt_write(const uint32_t channel, const char *buf, uint32_t len)
    {
        usbCdc_Logger(len, buf);
        return len;
    }
    /* host to target: read one character from the channel, non-blocking. return character, -1 if no character */
    /**
     *
     *
     */
    int32_t rtt_getchar(const uint32_t channel)
    {
        return -1;
    }
    /* host to target: true if no characters available for reading in the selected channel */
    /**
     *
     *
     */
    bool rtt_nodata(const uint32_t channel)
    {
        return true;
    }
}
// EOF
