// syncsocketserver.cpp
#include "esp_qtnetwork.h"
#include <QDebug>

SyncSocketServer::SyncSocketServer(QObject *parent)
    : QObject(parent), server(new QTcpServer(this)), clientSocket(nullptr)
{
}

SyncSocketServer::~SyncSocketServer()
{
    if (clientSocket)
    {
        clientSocket->disconnectFromHost();
        clientSocket->deleteLater();
    }
    server->close();
}

bool SyncSocketServer::startServer(quint16 port)
{
    if (!server->listen(QHostAddress::Any, port))
    {
        qWarning() << "Server could not start:" << server->errorString();
        return false;
    }
    qDebug() << "Server listening on port" << port;
    return true;
}

bool SyncSocketServer::waitForClient(int msec)
{
    qWarning() << "--waiting-- ";
    if (!server->waitForNewConnection(msec))
    {
        qWarning() << "No client connected within timeout";
        return false;
    }
    printf("Connected\n");
    clientSocket = server->nextPendingConnection();
    qWarning() << "Client connected from" << clientSocket->peerAddress().toString();
    return true;
}

QByteArray SyncSocketServer::readBytes(int maxSize)
{
    if (!clientSocket)
        return QByteArray();

    if (!clientSocket->waitForReadyRead(30000))
    {
        qWarning() << "Read timeout";
        return QByteArray();
    }
    return clientSocket->read(maxSize);
}

bool SyncSocketServer::writeBytes(const QByteArray &data, int &ow)
{
    if (!clientSocket)
        return false;

    qint64 written = clientSocket->write(data);
    if (written == -1)
    {
        qWarning() << "Write failed";
        return false;
    }
    if (!clientSocket->waitForBytesWritten(50000))
    {
        qWarning() << "Write timeout";
        return false;
    }
    ow = (int)written;
    return true;
}
