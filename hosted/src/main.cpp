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
BMPTcp *current_connection = NULL;
bool running = true;
//
//
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
        qInfo() << "Tcp Server started on port " << PORT;
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
	current_connection=this;
	rngdbstub_init();
}
//
//
//
void BMPTcp::disconnected()
{
	qInfo() << "Client disconnected";
	current_connection=NULL;
	rngdbstub_shutdown();
	this->deleteLater();
	running=false;
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
//
//
void BMPTcp::write( uint32_t sz, const uint8_t *ptr)
{
	_socket->write( (const char *)ptr,sz);
}
//
//
void BMPTcp::flush()
{
	_socket->flush();
}



extern "C"
{
 void         rngdb_send_data_c( uint32_t sz, const uint8_t *ptr)
 {
	qInfo() << "Reply :" << QString( (const QChar *)ptr,sz);
	if(current_connection)
	{
		current_connection->write(sz,ptr);
	}
 }
void rngdb_output_flush()
{
	if(current_connection)
	{
		current_connection->flush();
	}

}
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
	while(running)
	{
		QCoreApplication::processEvents();
	}	
	delete server;
	server=NULL;
	/* Should never get here */
	return 0;
}

