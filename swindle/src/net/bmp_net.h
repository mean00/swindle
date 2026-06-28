/**
 * @file bmp_net.h
 * @brief Network socket runner declarations for GDB and RTT.
 */
#pragma once
#include "esprit.h"
#include "lnSocketRunner.h"

/** @brief Socket runner for RTT data streaming (port 2001). */
class socketRunnerRtt : public socketRunner
{
  public:
    socketRunnerRtt(lnFastEventGroup &eventGroup, uint32_t shift);

  protected:
    bool _connected; /**< true when a debugger is connected. */
    virtual void hook_connected();
    virtual void hook_disconnected();
    virtual void hook_poll();
    void process_incoming_data(); /**< Drop all inbound RTT data. */
};

/** @brief Socket runner for the GDB stub protocol (port 2000). */
class socketRunnerGdb : public socketRunner
{
  public:
    socketRunnerGdb(lnFastEventGroup &eventGroup, uint32_t shift);
    virtual void hook_connected();
    virtual void hook_disconnected();
    virtual void hook_poll();

  protected:
    void process_incoming_data(); /**< Forward incoming data to the GDB stub. */
};

/** @name Global runner instances. */
/**@{*/
extern socketRunnerGdb *runnerGdb;
extern socketRunnerRtt *runnerRtt;
/**@}*/

void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl);
void NetCb_c(lnLwipEvent evt, void *arg);
