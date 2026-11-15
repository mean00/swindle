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
#include "hosted_probe.h"
#include "gdb_decoder.h"

extern "C"
{
#include "platform_support.h"
}

#define LN_ARCH LN_ARCH_ARM
#include "esp_socket_rust.h"
#include "lnFreeRTOS_pp.h"

#define RUNNER_GDB_PORT DEFAULT_GDB_TCP_PORT

extern void initTcpLayer();
extern "C" void platform_timeout_init();
extern "C" void platform_init(int argc, char **argv);

/**
 * @brief Background thread hosting the GDB TCP server.
 *
 * The thread itself is back-end agnostic: what sits behind the listening socket
 * is whatever runner @c factory builds (the BMP core for a USB probe, the relay
 * for a network probe — see hosted_probe.h).
 */
class GdbServerThread : public QThread
{
    Q_OBJECT

  public:
    explicit GdbServerThread(HostedRunnerFactory factory, QObject *parent = nullptr)
        : QThread(parent), m_factory(factory)
    {
    }

    bool isClientConnected() const
    {
        return m_runner != nullptr && hosted_runner_is_connected(m_runner);
    }

    void disconnectClient()
    {
        m_disconnectRequested = true;
        m_eventGroup.setEvents(RUNNER_DISCONNECTED);
    }

    void stopServer()
    {
        requestInterruption();
        m_eventGroup.setEvents(RUNNER_DOWN);
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
        m_runner = m_factory(m_eventGroup);
        if (m_runner == nullptr || !hosted_runner_start(m_runner))
        {
            fprintf(stderr, "[GDB Server] Cannot start the runner\n");
            hosted_runner_free(m_runner);
            return;
        }

        uint32_t mask = 0xffffffffUL;
        mask &= ~RUNNER_CAN_WRITE;
        const uint32_t global_mask = (RUNNER_UP | RUNNER_DOWN);

        printf("[GDB Server] Listening on localhost:%d\n", DEFAULT_GDB_TCP_PORT);

        bool last_connected = false;

        while (!isInterruptionRequested())
        {
            uint32_t events = m_eventGroup.waitEvents(mask, EVENT_POLL_TIMEOUT_MS);

            if (m_disconnectRequested)
            {
                m_disconnectRequested = false;
                hosted_runner_disconnect(m_runner);
            }

            uint32_t global_events = events & global_mask;
            uint32_t local_events = events & (~global_mask);
            uint32_t limited = local_events & RUNNER_SLOT_MASK;
            hosted_runner_process(m_runner, limited | global_events);
            hosted_runner_poll(m_runner);

            bool cur_connected = hosted_runner_is_connected(m_runner);
            if (cur_connected != last_connected)
            {
                last_connected = cur_connected;
                emit connectionStateChanged(cur_connected);
            }
        }

        target_list_free();
        hosted_runner_free(m_runner);
        m_runner = nullptr;
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

    HostedRunnerFactory m_factory;      /**< builds the probe side of the session. */
    void *m_runner{nullptr};            /**< Rust runner; NULL before run() / after it. */
    lnFastEventGroup m_eventGroup;
    bool m_disconnectRequested{false};
};
