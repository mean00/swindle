// syncsocketserver.cpp
#include "esp_qtnetwork.h"
#include <QDebug>
#include <QDateTime>
#include <QThread>

#define DEBUGME(...)                                                                                                   \
    {                                                                                                                  \
    }
void hexDump(bool wr, int n, const char *x)
{
    return;
    if (wr)
        printf(">>>>>: [");
    else
        printf("<<<<<<:[");
    for (int i = 0; i < n; i++)
    {
        const char c = x[i];
        if (c < ' ')
            printf(".");
        else
            printf("%c", c);
    }
    printf("]\n");
}

#define CHECK_THR()                                                                                                    \
    {                                                                                                                  \
        if (!(QThread::currentThread() == clientSocket->thread()))                                                     \
        {                                                                                                              \
            printf("** wrong thread**\n");                                                                             \
            exit(-1);                                                                                                  \
        }                                                                                                              \
    }

void SyncSocketServer::onNewConnection()
{
    qWarning("New incoming connection..\n");
    clientSocket = server->nextPendingConnection();
    if (clientSocket)
    {
        qWarning("Accepted ..\n");
        // connect(socket, &QTcpSocket::readyRead,    this, &EchoClient::onReadyRead);
        clientSocket->setParent(NULL);
        clientSocket->moveToThread(QThread::currentThread());
        clientSocket->setSocketOption(QAbstractSocket::LowDelayOption, 1);
        QObject::connect(clientSocket, &QTcpSocket::readyRead, this, &SyncSocketServer::dataAvailable);
        QObject::connect(clientSocket, &QTcpSocket::disconnected, this, &SyncSocketServer::onDisconnect);
        _parent->onNewConnection();
        qWarning("Accepted !\n");
    }
}

void SyncSocketServer::onDisconnect()
{
    qWarning("Disconnected signal \n");
    _parent->onDisconnect();
}
void SyncSocketServer::dataAvailable()
{

    DEBUGME("Data available ..\n");
    _parent->onDataAvailable();
}
//----------------------

SyncSocketServer::SyncSocketServer(lnSocketQt *parent)
    : QObject(NULL), server(new QTcpServer(this)), clientSocket(nullptr)
{
    _parent = parent;
    if (!QObject::connect(server, &QTcpServer::newConnection, this, &SyncSocketServer::onNewConnection))
    {
        printf("QObject::Connect failed \n");
        exit(-1);
    }
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
    qDebug() << "Server listening on port" << port << "\n";
    return true;
}

QByteArray SyncSocketServer::readBytes(int maxSize)
{
    if (!clientSocket)
        return QByteArray();

    CHECK_THR();
    QByteArray array = clientSocket->readAll();
    hexDump(false, array.size(), array.data());
    return array;
}

bool SyncSocketServer::writeBytes(const QByteArray &data, int &ow)
{
    CHECK_THR();
    if (!clientSocket)
        return false;
    // qWarning() << QDateTime::currentDateTime().toString("mm:ss:ms") << "Writing " << data.size() << "\n ";

    hexDump(true, data.size(), data.data());
    qint64 written = clientSocket->write(data);
    // qWarning() << "Wrote " << written << " bytes out of " << data.size() << "\n ";
    if (written == -1)
    {
        qWarning() << "Write failed";
        return false;
    }
    // more or less assume we wrote everything
    ow = (int)written;
    return true;
}
