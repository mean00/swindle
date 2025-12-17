/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/* Provides main entry point.  Initialise subsystems and enter GDB
 * protocol loop.
 */
#include <QCoreApplication>
#include <QDebug>
#include <QObject>
#include <QTimer>
#include <QThread>
#include <QtGlobal>
//
#include "lnLWIP.h"
//
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
//--
#define LN_ARCH LN_ARCH_ARM
#include "lnSocketRunner.h"
// Rust part
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
}
//-----

bool running = true;
extern void initTcpLayer();
//
//

void customHandler(QtMsgType type, const QMessageLogContext &context, const QString &msg)
{
    QByteArray localMsg = msg.toLocal8Bit();
    fprintf(stderr, "%s", localMsg.constData());
}
#define DEBUGME printf
//
//
//
//

void exit_from_bmp()
{
    QCoreApplication::exit(0);
}

extern "C" void rngdbstub_poll();
void trampoline()
{
    rngdbstub_poll();
    // printf("Pending : %d\n",server->hasPendingConnections());
}

class socketRunnerGdb : public socketRunner
{
  public:
    socketRunnerGdb(lnFastEventGroup &eventGroup, uint32_t shift) : socketRunner(2000, eventGroup, shift)
    {
        _connected = false;
    }

  protected:
    bool _connected;
    virtual void hook_connected()
    {
        // swindle_reinit_rtt();
        if (!_connected)
        {
            rngdbstub_init();
            // bmp_io_begin_session();
            _connected = true;
        } // ???????
    }
    virtual void hook_disconnected()
    {
        _connected = false;
        rngdbstub_shutdown();
        // bmp_io_end_session();
    }
    virtual void hook_poll()
    {
        if (_connected) // connected to a debugger
        {
            rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
#if 0
            if (cur_target)   // and we are connected to a target...
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
#endif
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
                DEBUGME("Processing %d bytes in gdb\n", rd);
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
socketRunnerGdb *runnerGdb = NULL;

/**
 * @brief [TODO:description]
 *
 * @param sz [TODO:parameter]
 * @param ptr [TODO:parameter]
 */
/**
 * @brief [TODO:description]
 */

extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    runnerGdb->writeData(sz, ptr);
}
extern "C" void rngdb_output_flush_c()
{
    runnerGdb->flushWrite();
}
extern "C" void platform_init(int argc, char **argv);

/*
 *
 */
void sys_network(lnLwipEvent evt, void *arg)
{
    printf("Got sys event %x\n,evt");
}

lnFastEventGroup network_eventGroup;
static void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl)
{
    uint32_t limited = (locl >> runner->shift()) & socketRunner::Mask;
    runner->process_events(limited | global);
}

class gdbThread : public QThread
{
  protected:
    void run() override
    {
        // QTimer mytimer;
        // QObject::connect(&mytimer, &QTimer::timeout, trampoline);
        // mytimer.start(100);
        // qInfo() << "Running in thread:" << QThread::currentThread();
        //
        const char *argv[2] = {"", NULL};
        platform_init((int)0, (char **)argv);
        initTcpLayer();
        lnLWIP::start(sys_network, NULL);
        network_eventGroup.takeOwnership();
        runnerGdb = new socketRunnerGdb(network_eventGroup, 0);
        runnerGdb->sendEvent(socketRunner::Up);
        uint32_t mask = (0xffffffffUL);
        mask &= ~(socketRunner::CanWrite << 0);
        const uint32_t global_mask = (socketRunner::Up | socketRunner::Down);
        while (1)
        {
            uint32_t events = network_eventGroup.waitEvents(mask, 20);
            uint32_t global_events = events & global_mask;
            uint32_t local_events = events & (~global_mask);
            process_sockets(runnerGdb, global_events, local_events);
        }
    }
};

int main(int argc, char **argv)
{
    qWarning() << "======================";
    qWarning() << "* Qt Swindle Hosted  *";
    qWarning() << "======================";
    QCoreApplication a(argc, argv);
    qInstallMessageHandler(customHandler);

    // go!
    gdbThread *t = new gdbThread;
    t->start();
    a.exec();
    return 0;
}
extern "C" void rv_test(void);
extern "C" void bmp_test(void)
{
    //    rv_test();
}
