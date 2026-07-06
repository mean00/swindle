/**
 * @file main.cpp
 * @brief Hosted-mode entry point: GDB server loop over TCP or RTT
 *
 * Original license from Black Magic Debug project:
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

#include <QCoreApplication>
#include <QDebug>
#include <QObject>
#include <QThread>
#include <QTimer>
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

// Defines expected by bmp_net_gdb.h before inclusion
#define RUNNER_GDB_PORT 2000
#define DEBUGME printf

// Use the canonical socketRunnerGdb from the firmware tree
#include "../../swindle/src/net/bmp_net_gdb.h"

//
//-----

//
bool running = true;
extern void initTcpLayer();
//
//

/**
 * @brief Custom Qt message handler that redirects to stderr.
 */
void customHandler(QtMsgType type, const QMessageLogContext &context, const QString &msg)
{
    QByteArray localMsg = msg.toLocal8Bit();
    fprintf(stderr, "%s", localMsg.constData());
}
//
//
//
//

/**
 * @brief Exit the Qt application (called from BMP core on shutdown).
 */
void exit_from_bmp()
{
    QCoreApplication::exit(0);
}

/**
 * @brief Trampoline called periodically to poll the GDB stub.
 */
void trampoline()
{
    rngdbstub_poll();
    // printf("Pending : %d\n",server->hasPendingConnections());
}

extern "C" void platform_init(int argc, char **argv);

/**
 * @brief lwIP system event callback (stub for hosted build).
 * @param evt  lwIP event type.
 * @param arg  User argument.
 */
void sys_network(lnLwipEvent evt, void *arg)
{
    printf("Got sys event %x\n,evt");
}

lnFastEventGroup network_eventGroup;

/**
 * @brief Process socket events for a given runner.
 * @param runner   The socket runner to process.
 * @param global   Global events (Up/Down).
 * @param locl     Local events (per-runner).
 */
void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl)
{
    uint32_t limited = (locl >> runner->shift()) & socketRunner::Mask;
    runner->process_events(limited | global);
}

/**
 * @brief GDB server thread.
 *
 * Initialises the platform, starts the TCP server, and runs the
 * event loop that processes socket events for the GDB stub.
 */
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

/**
 * @brief Application entry point.
 *
 * Installs a custom message handler, starts the GDB thread,
 * and enters the Qt event loop.
 * @param argc  Argument count.
 * @param argv  Argument vector.
 * @return Application exit code.
 */
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

/**
 * @brief BMP test stub (unused).
 */
extern "C" void rv_test(void);
extern "C" void bmp_test(void)
{
    //    rv_test();
}

#include "bmp_net_gdb.cpp"
