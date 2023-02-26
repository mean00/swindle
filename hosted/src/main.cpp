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
#include "qtcp.h"

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
#define PORT 2000

BmpTcpServer::BmpTcpServer(QObject *parent )
{
	_server = new QTcpServer(this);
	connect(_server, SIGNAL(newConnection()),  this, SLOT(newConnection()));
	if(!_server->listen(QHostAddress::Any, PORT))
    {
        qInfo() << "Tcp Server could not start";
    }
    else
    {
        qInfo() << "Tcp Server started!";
    }
}
    
void BmpTcpServer::newConnection()
{
	qInfo() << "New connection";
 	BMPTcp *tcp  = new BMPTcp(_server->nextPendingConnection());
}
//
//
//

BMPTcp::BMPTcp(QTcpSocket *sock)
{
	_socket = sock;
	connect(_socket, SIGNAL(disconnected()),  this, SLOT(disconnected()));
	connect(_socket, SIGNAL(readyRead()),  this, SLOT(readyRead()));
	rngdbstub_init();
}
//
//
//
void BMPTcp::disconnected()
{
	qInfo() << "Client disconnected";
	rngdbstub_shutdown();
	this->deleteLater();
}
//
//
//
void BMPTcp::readyRead()
{
	while(true)
	{
		int nb= _socket->bytesAvailable();
		if (nb<=0)
			return;		
		if(nb>QBUFFER_SIZE) nb=QBUFFER_SIZE;		
		int actual = _socket->read((char *)_buffer, nb);
		rngdbstub_run(actual,_buffer);

	}
}


//-


#define BUF_SIZE 1024U
static char pbuf[BUF_SIZE + 1U];

extern "C"
{
 void         rngdb_send_data_c( uint32_t sz, const uint8_t *ptr)
 {
	for(int i=0;i<sz;i++)
	{
		gdb_if_putchar(ptr[i], 1);
	}
 }
void rngdb_output_flush()
{

}
}
static void bmp_poll_loop(void)
{
	uint8_t c=gdb_if_getchar();
	rngdbstub_run(1,&c);
}
//
//
//
//
int main(int argc, char **argv)
{
	qInfo() << "Qt BMP started";
	QCoreApplication a(argc, argv);

	platform_init(argc, argv);	
	BmpTcpServer *server = new BmpTcpServer;
	while(1)
	{
		QCoreApplication::processEvents();
	}
	SET_IDLE_STATE(true);
	while (true) {
		volatile struct exception e;
		TRY_CATCH(e, EXCEPTION_ALL) {
			bmp_poll_loop();
		}
		if (e.type) {
			gdb_putpacketz("EFF");
			target_list_free();
			morse("TARGET LOST.", 1);
		}
	}
	delete server;
	server=NULL;
	/* Should never get here */
	return 0;
}

