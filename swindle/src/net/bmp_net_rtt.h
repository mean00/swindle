/*

 */
#include "esprit.h"

extern "C"
{
#include "exception.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "target.h"
#include "version.h"
}
#include "lnLWIP.h"
//
//
//
// device -> host
// host -> device
extern "C" void bmp_rtt_poll_c();
extern "C" void swindle_init_rtt();
extern "C" void swindle_run_rtt();
extern "C" void swindle_purge_rtt();
extern "C" bool swindle_rtt_enabled();
// device -> host
extern "C" uint32_t usbCdc_write_available();
extern "C" bool swindle_write_rtt_channel(uint32_t channel, uint32_t size, const uint8_t *data);
// host -> device
extern "C" void swindle_reinit_rtt();
extern "C" uint32_t swindle_rtt_write_available(uint32_t channel);
/*
 *
 *
 */
/*
 *
 *
 */
class socketRunnerRtt : public socketRunner
{
  public:
    socketRunnerRtt(lnFastEventGroup &eventGroup, uint32_t shift) : socketRunner(RUNNER_RTT_PORT, eventGroup, shift)
    {
        Logger("RunnerRtt...\n");
        _connected = false;
        swindle_init_rtt();
    }

  protected:
    bool _connected;
    virtual void hook_connected()
    {
        swindle_reinit_rtt();
        _connected = true;
    }
    virtual void hook_disconnected()
    {
        _connected = false;
    }
    virtual void hook_poll()
    {
        if (_connected) // connected to a debugger
        {
            if (cur_target) // and we are connected to a target...
            {
                if (swindle_rtt_enabled())
                {
                    swindle_run_rtt();
                }
                else
                {
                    swindle_purge_rtt();
                }
            }
        }
    }

  protected:
    // drop all data incoming for rtt
    void process_incoming_data()
    {
        uint32_t lp = 0;

        while (1)
        {
            uint32_t rd = 0;
            uint8_t *data;
            if (readData(rd, &data))
            {
                if (!rd)
                    return;
                releaseData();
            }
        }
    }

  protected:
};
/**/
socketRunnerRtt *runnerRtt = NULL;
/**
 * @brief [TODO:description]
 *
 * @param index [TODO:parameter]
 * @param len [TODO:parameter]
 * @param data [TODO:parameter]
 */
extern "C" void swindle_rtt_send_data_to_host(unsigned int index, uint32_t len, const uint8_t *data)
{
    xAssert(index == 0);
    runnerRtt->writeData(len, data);
    runnerRtt->flushWrite();
}
/**
 * @brief [TODO:description]
 *
 * @param dex [TODO:parameter]
 * @return [TODO:return]
 */
extern "C" uint32_t swindle_rtt_room_available_to_host(uint32_t dex)
{
    xAssert(dex == 0);
    return runnerRtt->writeBufferAvailable();
}

// EOF
