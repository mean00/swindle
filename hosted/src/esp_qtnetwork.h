
// syncsocketserver.h
#ifndef SYNCSOCKETSERVER_H
#define SYNCSOCKETSERVER_H

#include <QTcpServer>
#include <QTcpSocket>
#include <QObject>
#include <QByteArray>
//
#include "lnLWIP.h"
class SyncSocketServer : public QObject
{
    Q_OBJECT

  public:
    explicit SyncSocketServer(QObject *parent = nullptr);
    ~SyncSocketServer();

    bool startServer(quint16 port = 2000);

    // Synchronous API
    bool waitForClient(int msec = 30000); // wait for a client connection
    QByteArray readBytes(int maxSize);
    bool writeBytes(const QByteArray &data, int &written);

  private:
    QTcpServer *server;
    QTcpSocket *clientSocket;
};
class lnSocketQt : public lnSocket
{
  public:
    virtual ~lnSocketQt() {};
    virtual lnSocket::status write(uint32_t n, const uint8_t *data, uint32_t &done);
    virtual lnSocket::status read(uint32_t n, uint8_t *data, uint32_t &done);
    virtual lnSocket::status flush();
    virtual lnSocket::status close();
    virtual lnSocket::status accept();

    lnSocketQt(uint16_t port, lnSocketCb cb, void *arg)
    {
        _port = port;
        _cb = cb;
        _arg = arg;
        _server = new SyncSocketServer();
    }

  protected:
    uint16_t _port;
    lnSocketCb _cb;
    void *_arg;

  public:
    SyncSocketServer *_server;
};

#endif // SYNCSOCKETSERVER_H
