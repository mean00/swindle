/**
 * @file esp_qtnetwork.h
 * @brief Qt-based TCP socket server and lnSocketQt class declarations.
 *
 * Provides a synchronous TCP server (SyncSocketServer) wrapping Qt's
 * asynchronous QTcpServer/QTcpSocket, and the lnSocketQt subclass
 * that bridges it to the debugger's abstract socket interface.
 */
// syncsocketserver.h
#pragma once
#include <QTcpServer>
#include <QTcpSocket>
#include <QObject>
#include <QByteArray>
//
#include "lnLWIP.h"

class lnSocketQt;

/**
 * @brief Synchronous TCP server wrapping Qt's asynchronous networking.
 *
 * Listens on a TCP port and provides blocking read/write operations
 * by processing Qt events internally.
 */
class SyncSocketServer : public QObject
{
    Q_OBJECT

  public:
    /**
     * @brief Construct a SyncSocketServer.
     * @param parent  The owning lnSocketQt instance.
     */
    explicit SyncSocketServer(lnSocketQt *parent);
    ~SyncSocketServer();

    /**
     * @brief Start listening on the given port.
     * @param port  TCP port number (default 2000).
     * @return true on success.
     */
    bool startServer(quint16 port = 2000);

    // Synchronous API
    /**
     * @brief Read all available bytes from the connected client.
     * @param maxSize  Maximum bytes to read (unused, reads all).
     * @return Byte array of received data.
     */
    QByteArray readBytes(int maxSize);

    /**
     * @brief Write data to the connected client.
     * @param data     Byte array to send.
     * @param written  Output: bytes actually written.
     * @return true on success.
     */
    bool writeBytes(const QByteArray &data, int &written);

    /**
     * @brief Flush the client socket's write buffer.
     */
    void flush()
    {
        clientSocket->flush();
    }

    /**
     * @brief Disconnect and delete the client socket.
     */
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
    void onDisconnect();
    void dataAvailable();
};

/**
 * @brief Qt-based implementation of the abstract lnSocket interface.
 *
 * Creates a SyncSocketServer and bridges socket events to the
 * debugger's callback-based event model.
 */
class lnSocketQt : public lnSocket
{
  public:
    virtual ~lnSocketQt() {};
    virtual status write(uint32_t n, const uint8_t *data, uint32_t &done);
    virtual status read(uint32_t &n, uint8_t **data);

    /**
     * @brief Invoke the socket event callback.
     * @param evt  The socket event.
     * @return lnSocket::Ok.
     */
    virtual status invoke(lnSocketEvent evt)
    {
        _cb(evt, _arg);
        return lnSocket::Ok;
    }

    /**
     * @brief Called when data is available from the client.
     */
    void onDataAvailable()
    {
        // printf("Data!\n");
        _cb(SocketDataAvailable, _arg);
    };

    /**
     * @brief Called when the client disconnects.
     */
    void onDisconnect()
    {
        _cb(SocketDisconnect, _arg);
    };

    /**
     * @brief Called when a new client connects.
     */
    void onNewConnection()
    {
        _cb(SocketConnected, _arg);
    };

    virtual status flush();
    virtual status disconnectClient();
    virtual status asyncMode();
    virtual status accept();
    virtual status freeReadData();

    /**
     * @brief Query available write buffer space.
     * @param n  Output: available bytes.
     * @return lnSocket::Ok.
     */
    virtual status writeBufferAvailable(uint32_t &n);

    /**
     * @brief Construct an lnSocketQt.
     * @param port  TCP port to listen on.
     * @param cb    Socket event callback.
     * @param arg   User argument for the callback.
     */
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
    QByteArray _readBuffer;

  public:
    SyncSocketServer *_server;
};