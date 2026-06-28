#pragma once
#include "esprit.h"
#include "lnSocketRunner.h"

/*
 *
 *
 */
class socketRunnerRtt : public socketRunner
{
  public:
    socketRunnerRtt(lnFastEventGroup &eventGroup,
                    uint32_t shift); // : socketRunner(RUNNER_RTT_PORT, eventGroup, shift);

  protected:
    bool _connected;
    virtual void hook_connected();
    virtual void hook_disconnected();
    virtual void hook_poll();

  protected:
    // drop all data incoming for rtt
    void process_incoming_data();
};
/**/

class socketRunnerGdb : public socketRunner
{
  public:
    socketRunnerGdb(lnFastEventGroup &eventGroup,
                    uint32_t shift); // : socketRunner(RUNNER_GDB_PORT, eventGroup, shift);
    virtual void hook_connected();
    virtual void hook_disconnected();
    virtual void hook_poll();

  protected:
    void process_incoming_data();

  protected:
};
/**/
extern socketRunnerGdb *runnerGdb;
extern socketRunnerRtt *runnerRtt;
//
void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl);
void NetCb_c(lnLwipEvent evt, void *arg);
