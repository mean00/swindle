/**
 * @file gdb_server_thread.h
 * @brief Background thread hosting the GDB TCP server for hosted mode.
 */

#pragma once

#include <QThread>
#include <cstdint>
#include <cstdio>
#include <cstdlib>

#include "lnLWIP.h"
#include "hosted_constants.h"
#include "gdb_decoder.h"

extern "C"
{
#include "exception.h"
#include "gdb_if.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "morse.h"
#include "target.h"
}

#define LN_ARCH LN_ARCH_ARM
#include "lnSocketRunner.h"
#include "../../swindle/src/net/bmp_net.h"

#define RUNNER_GDB_PORT DEFAULT_GDB_TCP_PORT

extern void initTcpLayer();
extern "C" void platform_timeout_init();
extern "C" void platform_init(int argc, char **argv);

/**
 * @brief Background thread hosting the GDB TCP server.
 */
class GdbServerThread : public QThread
{
    Q_OBJECT

  public:
    GdbServerThread(QObject *parent = nullptr) : QThread(parent) {}

    bool isClientConnected() const
    {
        return runnerGdb && runnerGdb->isConnected();
    }

    void disconnectClient()
    {
        m_disconnectRequested = true;
        m_eventGroup.setEvents(socketRunner::Disconnected);
    }

    void stopServer()
    {
        requestInterruption();
        m_eventGroup.setEvents(socketRunner::Down);
    }

  signals:
    void connectionStateChanged(bool connected);

  protected:
    void run() override
    {
        const char *argv[2] = {"", nullptr};
        platform_init(0, const_cast<char **>(argv));
        initTcpLayer();
        lnLWIP::start(sys_network, nullptr);

        m_eventGroup.takeOwnership();
        runnerGdb = new socketRunnerGdb(m_eventGroup, 0);
        runnerGdb->sendEvent(socketRunner::Up);

        uint32_t mask = 0xffffffffUL;
        mask &= ~(socketRunner::CanWrite << 0);
        const uint32_t global_mask = (socketRunner::Up | socketRunner::Down);

        printf("[GDB Server] Listening on localhost:%d\n", DEFAULT_GDB_TCP_PORT);

        bool last_connected = false;

        while (!isInterruptionRequested())
        {
            uint32_t events = m_eventGroup.waitEvents(mask, EVENT_POLL_TIMEOUT_MS);

            if (m_disconnectRequested)
            {
                m_disconnectRequested = false;
                if (runnerGdb)
                {
                    runnerGdb->disconnectClient();
                }
            }

            uint32_t global_events = events & global_mask;
            uint32_t local_events = events & (~global_mask);
            uint32_t limited = (local_events >> runnerGdb->shift()) & socketRunner::Mask;
            runnerGdb->process_events(limited | global_events);

            bool cur_connected = runnerGdb->isConnected();
            if (cur_connected != last_connected)
            {
                last_connected = cur_connected;
                emit connectionStateChanged(cur_connected);
            }
        }

        target_list_free();
        if (runnerGdb)
        {
            delete runnerGdb;
            runnerGdb = nullptr;
        }
        if (last_connected)
        {
            emit connectionStateChanged(false);
        }
        printf("[GDB Server] Stopped.\n");
    }

  private:
    static void sys_network(lnLwipEvent evt, void *arg)
    {
        (void)arg;
        printf("[Network] lwIP system event: 0x%x\n", static_cast<unsigned int>(evt));
    }

    lnFastEventGroup m_eventGroup;
    bool m_disconnectRequested{false};
};
