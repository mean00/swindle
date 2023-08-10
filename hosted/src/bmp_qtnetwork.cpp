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
#include "bmp_qtnetwork.h"
#include "bmp_logger.h"
#include <QCoreApplication>
#include <QDateTime>
#include <QObject>
#include <QTimer>
#include <QtGlobal>

extern "C"
{
#include "exception.h"
#include "gdb_if.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "morse.h"
#include "target.h"
#include <fcntl.h>
}
//-- disable debug
#if 1
#undef QBMPLOG
#undef QBMPLOGN
#define QBMPLOG(...)                                                                                                   \
    {                                                                                                                  \
    }
#define QBMPLOGN(...)                                                                                                  \
    {                                                                                                                  \
    }
#endif    
// Rust part
extern "C"
{
    void rngdbstub_init();
    void rngdbstub_shutdown();
    void rngdbstub_run(uint32_t s, const uint8_t *d);
}
//-----
#define PORT 2000
BMPTcp *current_connection = NULL;

extern void exit_from_bmp();
#define BUFFER_SIZE (3*1024)
char tcp_buffer[BUFFER_SIZE];
int tcp_index = 0;

void lowDelay(int skt)
{
    int flags = fcntl(skt, F_GETFL, 0);
    if (fcntl(skt, F_SETFL, flags | O_NDELAY) < 0)
    {
        printf("Cannot set nodelay\n");
        exit(-1);
    }
}

//
//
BmpTcpServer::BmpTcpServer(QObject *parent)
{
    _server = new QTcpServer(this);
    connect(_server, SIGNAL(newConnection()), this, SLOT(newConnection()));
    if (!_server->listen(QHostAddress::AnyIPv4, PORT))
    {
        QBMPLOG("**Tcp Server could not start**\n");
        exit_from_bmp();
        QCoreApplication::quit();
    }
    else
    {
        // swith to low delay using low level call
        lowDelay(_server->socketDescriptor());
        QBMPLOG("Tcp Server started on port %d \n", PORT);
    }
}

void BmpTcpServer::newConnection()
{
    QBMPLOG("New connection !!\n");
    BMPTcp *tcp = new BMPTcp(_server->nextPendingConnection());
}
//
//
//

BMPTcp::BMPTcp(QTcpSocket *sock)
{
    _socket = sock;

    connect(_socket, SIGNAL(disconnected()), this, SLOT(disconnected()));
    connect(_socket, SIGNAL(readyRead()), this, SLOT(readyRead()));
    current_connection = this;
    _socket->setReadBufferSize(512);
    _socket->setSocketOption(QAbstractSocket::LowDelayOption, 1);
    lowDelay(_socket->socketDescriptor());

    rngdbstub_init();
}
//
//
//
void BMPTcp::disconnected()
{
    qInfo() << "Client disconnected";
    current_connection = NULL;
    rngdbstub_shutdown();
    this->deleteLater();
    exit_from_bmp();
}
//
//
//
void BMPTcp::readyRead()
{
    QBMPLOG("--Ready data available\n");
    while (true)
    {
        int nb = _socket->bytesAvailable();
        if (nb <= 0)
        {
            QBMPLOG("--data processed\n");
            return;
        }
        if (nb > QBUFFER_SIZE)
            nb = QBUFFER_SIZE;
        int actual = _socket->read((char *)_buffer, nb);
        QBMPLOG("tcp read \n");
        QBMPLOGN((int)actual, (char *)_buffer);
        rngdbstub_run(actual, _buffer);
    }
}
//
//
void BMPTcp::write(uint32_t sz, const uint8_t *ptr)
{
    memcpy(tcp_buffer + tcp_index, ptr, sz);
    tcp_index += sz;
    if (tcp_index >= BUFFER_SIZE)
    {
        printf("** BUFFER OVERFLOW **\n");
        exit(-1);
    }
}
//
//
void BMPTcp::flush()
{
    if(!tcp_index) return;
    QBMPLOG("tcp write :\n");
#if 0    
    for (int i = 0; i < tcp_index; i++)
    {
        uint8_t c = tcp_buffer[i];
    }
#endif    
    QBMPLOGN((int)tcp_index, (const char *)tcp_buffer);
    QBMPLOG("\n--\n");
    if (tcp_index != _socket->write(tcp_buffer, tcp_index))
    {
        printf("** incomplete send **\n");
        exit(-1);
    }
    tcp_index = 0;
    _socket->flush();
    _socket->waitForBytesWritten();
}

static bool eol = true;

extern "C"
{
    /*
     */
    void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
    {
        if (eol)
        {
            eol = false;
            QBMPLOG("Reply :\n");
        }
#if 0
    for(int i=0;i<sz;i++)
    {
        printf("0x%02x-",ptr[i]);
    }
    printf("!");
    for(int i=0;i<sz;i++)
    {
        int c=ptr[i];
        if(c<' ') c='.';
        printf("%c!",c);
    }
#endif

        if (current_connection)
        {
            current_connection->write(sz, ptr);
        }
    }
    /*
     */
    void rngdb_output_flush_c()
    {
        if (current_connection)
        {
            QBMPLOG(" flush \n");
            current_connection->flush();
            current_connection->flush();
        }
        QBMPLOG("\n");
        eol = true;
    }
}
