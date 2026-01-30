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
// Rust part
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
    void rngdbstub_poll();
}
//
//
//
// device -> host
// host -> device
/*
 *
 *
 */
class socketRunnerGdb : public socketRunner
{
  public:
    socketRunnerGdb(lnFastEventGroup &eventGroup, uint32_t shift) : socketRunner(RUNNER_GDB_PORT, eventGroup, shift)
    {
        _connected = false;
        Logger("RunnerGdb..\n");
    }

  protected:
    bool _connected;
    virtual void hook_connected()
    {
        rngdbstub_init();
        bmp_io_begin_session();
        _connected = true;
    }
    virtual void hook_disconnected()
    {
        _connected = false;
        rngdbstub_shutdown();
        bmp_io_end_session();
    }
    virtual void hook_poll()
    {
        if (_connected) // connected to a debugger
        {
            rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
        }
    }

  protected:
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
                rngdbstub_run(rd, data);
                releaseData();
                DEBUGME("\td%d\n", lp++);
            }
        }
    xit:
        flushWrite();
    }

  protected:
};
/**/
socketRunnerGdb *runnerGdb = NULL;
/**
 * @brief [TODO:description]
 *
 * @param sz [TODO:parameter]
 * @param ptr [TODO:parameter]
 */
extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    runnerGdb->writeData(sz, ptr);
}
/**
 * @brief [TODO:description]
 */
extern "C" void rngdb_output_flush_c()
{
    runnerGdb->flushWrite();
}
// EOF
