/**
 * @file
 * @brief [TODO:description]
 *
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
bool lnLWIP::start(lnLwIpSysCallback cb, void *arg)
{
    _sys_cb = cb;
    _sys_arg = arg;
    _sys_cb(LwipReady, arg);
    return true;
};

static QByteArray _sbytes;
lnSocket::status lnSocketQt::read(uint32_t &n, uint8_t **data)
{
    DEBUGME("Trying to read...\n");
    _sbytes = _server->readBytes(n);
    *data = (uint8_t *)_sbytes.data();
    n = _sbytes.size();
    if (n)
    {
        // qWarning() << QDateTime::currentDateTime().toString("mm:ss:ms") << "Got  bytes " << n << "...\n";
    }
    return lnSocket::Ok;
}
/**
 */
lnSocket::status lnSocketQt::flush()
{
    _server->flush();
    return lnSocket::Ok;
}
/**
 */
lnSocket::status lnSocketQt::disconnectClient()
{
    _server->disconnect();
    return lnSocket::Ok;
}
/**
 */
lnSocket::status lnSocketQt::asyncMode()
{
    return lnSocket::Ok;
}
/**
 */
lnSocket::status lnSocketQt::accept()
{
    return lnSocket::Error;
}
/**
 */
lnSocket::status lnSocketQt::freeReadData()
{
    return lnSocket::Ok;
}
/**
 */
lnSocket::status lnSocketQt::write(uint32_t n, const uint8_t *data, uint32_t &done)
{
    QByteArray bytes((const char *)data, (int)n);
    int o = 0;
    _server->writeBytes(bytes, o);
    done = o;
    return lnSocket::Ok;
}
void initTcpLayer()
{
}
//
