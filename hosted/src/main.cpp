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
extern lnSocket *current_connection;
int main(int argc, char **argv)
{
    qInfo() << "======================";
    qInfo() << "* Qt Swindle Hosted  *";
    qInfo() << "======================";
    QCoreApplication a(argc, argv);
    qInstallMessageHandler(customHandler);
    platform_init(argc, argv);
    initTcpLayer();

    QTimer mytimer;
    QObject::connect(&mytimer, &QTimer::timeout, trampoline);
    mytimer.start(100);
    // go!
    rngdbstub_init();
    uint8_t buffer[2048];
    // QCoreApplication::exec();
    current_connection->accept();
    while (1)
    {
        uint32_t n = 0;
        if (lnSocket::Ok == current_connection->read(512, buffer, n))
        {
            rngdbstub_run(n, buffer);
        }
    }
    // should never get here
    return 0;
}
extern "C" void rv_test(void);
extern "C" void bmp_test(void)
{
    //    rv_test();
}
