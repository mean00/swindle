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
#include <QObject>
#include <QCoreApplication>
#include <QtGlobal>
#include <QTimer>
#include "bmp_qtnetwork.h"

extern "C"
{
#include "general.h"
#include "gdb_if.h"
#include "gdb_main.h"
#include "target.h"
#include "exception.h"
#include "gdb_packet.h"
#include "morse.h"
}
//--

// Rust part
extern "C" 
{
	void rngdbstub_init();
	void rngdbstub_shutdown() ;
	void rngdbstub_run(uint32_t s, const uint8_t *d);
}
//-----

bool running = true;
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
	running=false;
}

extern "C" void rngdbstub_poll();
int main(int argc, char **argv)
{
	qInfo() << "Qt BMP started";
	QCoreApplication a(argc, argv);
	qInstallMessageHandler(customHandler);
	platform_init(argc, argv);	
	BmpTcpServer *server = new BmpTcpServer;

 	QTimer mytimer;
    QObject::connect(&mytimer,&QTimer::timeout,rngdbstub_poll);
    mytimer.start(100);

	while(running)
	{
		QCoreApplication::processEvents( QEventLoop::AllEvents, 50);
	}	
	delete server;
	server=NULL;
	/* Should never get here */
	return 0;
}


