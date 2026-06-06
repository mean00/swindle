/**
 * @file esp_bridge.cpp
 * @brief Qt-based TCP socket bridge for the hosted debugger.
 *
 * Provides the lnSocket and lnLWIP implementations using Qt networking
 * (QTcpServer / QTcpSocket) for the hosted (non-embedded) build.
 */

#include "bmp_logger.h"
#include <QCoreApplication>
#include <QDateTime>
#include <QObject>
#include <QTimer>
#include <QtGlobal>
//
#include "lnLWIP.h"
//
#include "esp_qtnetwork.h"

#include "lnFreeRTOS_pp.h"
#define xxAssert(c)                                                                                                    \
    if (!(c))                                                                                                          \
    {                                                                                                                  \
        printf("** %s fail! at line %d file %s\n", #c, __LINE__, __FILE__);                                            \
        exit(-1);                                                                                                      \
    }
//
#define DEBUGME(...)                                                                                                   \
    {                                                                                                                  \
    }
//
lnLwIpSysCallback _sys_cb;
void *_sys_arg;

/**
 * @brief Create a TCP socket server on the given port.
 * @param port  TCP port to listen on.
 * @param cb    Callback for socket events.
 * @param arg   User argument passed to the callback.
 * @return Pointer to the created lnSocket, or NULL on failure.
 */
lnSocket *lnSocket::create(uint16_t port, lnSocketCb cb, void *arg)
{
    lnSocketQt *q = new lnSocketQt(port, cb, arg);
    if (!q->_server->startServer(port))
    {
        printf("!! Cannot start server\n");
        return NULL;
    }
    printf("Server started on port :%d\n", port);
    return q;
}

/**
 * @brief Start the lwIP network layer (stub for hosted build).
 * @param cb  Callback for lwIP system events.
 * @param arg User argument.
 * @return true on success.
 */
bool lnLWIP::start(lnLwIpSysCallback cb, void *arg)
{
    _sys_cb = cb;
    _sys_arg = arg;
    _sys_cb(LwipReady, arg);
    return true;
};

/**
 * @brief Read available data from the connected client.
 * @param n     Output: number of bytes read.
 * @param data  Output: pointer to the read data buffer.
 * @return lnSocket::Ok on success.
 */
lnSocket::status lnSocketQt::read(uint32_t &n, uint8_t **data)
{
    DEBUGME("Trying to read...\n");
    _readBuffer = _server->readBytes(n);
    *data = (uint8_t *)_readBuffer.data();
    n = _readBuffer.size();
    if (n)
    {
        // qWarning() << QDateTime::currentDateTime().toString("mm:ss:ms") << "Got  bytes " << n << "...\n";
    }
    return lnSocket::Ok;
}

/**
 * @brief Flush the socket write buffer.
 * @return lnSocket::Ok on success.
 */
lnSocket::status lnSocketQt::flush()
{
    _server->flush();
    return lnSocket::Ok;
}

/**
 * @brief Disconnect the connected client.
 * @return lnSocket::Ok on success.
 */
lnSocket::status lnSocketQt::disconnectClient()
{
    _server->disconnect();
    return lnSocket::Ok;
}

/**
 * @brief Switch to asynchronous mode (stub, always succeeds).
 * @return lnSocket::Ok.
 */
lnSocket::status lnSocketQt::asyncMode()
{
    return lnSocket::Ok;
}

/**
 * @brief Accept a new connection (not supported in hosted build).
 * @return lnSocket::Error.
 */
lnSocket::status lnSocketQt::accept()
{
    return lnSocket::Error;
}

/**
 * @brief Free the read data buffer (stub, no-op).
 * @return lnSocket::Ok.
 */
lnSocket::status lnSocketQt::freeReadData()
{
    return lnSocket::Ok;
}

/**
 * @brief Write data to the connected client.
 * @param n     Number of bytes to write.
 * @param data  Pointer to the data.
 * @param done  Output: number of bytes actually written.
 * @return lnSocket::Ok on success.
 */
lnSocket::status lnSocketQt::write(uint32_t n, const uint8_t *data, uint32_t &done)
{
    QByteArray bytes((const char *)data, (int)n);
    int o = 0;
    _server->writeBytes(bytes, o);
    done = o;
    return lnSocket::Ok;
}

/**
 * @brief Query how many bytes can be written without blocking.
 * @param n  Output: available buffer space.
 * @return lnSocket::Ok.
 */
lnSocket::status lnSocketQt::writeBufferAvailable(uint32_t &n)
{
    n = 2048; // assume we can buffer a lot and blocking is not a problem
    return lnSocket::Ok;
}

/**
 * @brief Initialise the TCP layer (stub for hosted build).
 */
void initTcpLayer()
{
}
//