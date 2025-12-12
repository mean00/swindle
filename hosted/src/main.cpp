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
    socketRunnerGdb()
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

/*
 *
 */
void sys_network(lnLwipEvent evt, void *arg)
{
    printf("Got sys event %x\n,evt");
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
        char *argv[2] = {"", ""};
        platform_init((int)0, argv);
        initTcpLayer();
        lnLWIP::start(sys_network, NULL);
        rngdbstub_init();
        runnerGdb = new socketRunnerGdb();
        runnerGdb->sendEvent(socketRunner::Up);
        runnerGdb->run();
        qWarning() << "Thread work done";
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
