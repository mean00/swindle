
// syncsocketserver.h
#pragma once
#include <QTcpServer>
#include <QTcpSocket>
#include <QObject>
#include <QByteArray>
//
#include "lnLWIP.h"

class lnSocketQt;
class SyncSocketServer : public QObject
{
    Q_OBJECT

  public:
    explicit SyncSocketServer(lnSocketQt *parent);
    ~SyncSocketServer();

    bool startServer(quint16 port = 2000);

    // Synchronous API
    QByteArray readBytes(int maxSize);
    bool writeBytes(const QByteArray &data, int &written);

    void flush()
    {
        clientSocket->flush();
    }
    void disconnect()
    {
        if (clientSocket)
        {
            delete clientSocket;
            clientSocket = NULL;
        }
    }

  private:
    QTcpServer *server;
    QTcpSocket *clientSocket;
    lnSocketQt *_parent;
  private slots:
    void onNewConnection();
    void dataAvailable();
};
/*

*/
class lnSocketQt : public lnSocket
{
  public:
    virtual ~lnSocketQt() {};
    virtual status write(uint32_t n, const uint8_t *data, uint32_t &done);
    virtual status read(uint32_t &n, uint8_t **data);
    virtual status invoke(lnSocketEvent evt)
    {
        _cb(evt, _arg);
        return lnSocket::Ok;
    }
    void onDataAvailable()
    {
        // printf("Data!\n");
        _cb(SocketDataAvailable, _arg);
    };
    void onNewConnection()
    {
        _cb(SocketConnected, _arg);
    };
    virtual status flush();
    virtual status disconnectClient();
    virtual status asyncMode();
    virtual status accept();
    virtual status freeReadData();

    lnSocketQt(uint16_t port, lnSocketCb cb, void *arg)
    {
        _port = port;
        _cb = cb;
        _arg = arg;
        _server = new SyncSocketServer(this);
    }

  protected:
    uint16_t _port;
    lnSocketCb _cb;
    void *_arg;

  public:
    SyncSocketServer *_server;
};
