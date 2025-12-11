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
/**
 */
lnSocket *lnSocket::create(uint16_t port, lnSocketCb cb, void *arg)
{
    xAssert(0);
    return NULL;
}
bool lnLWIP::start(lnLwIpSysCallback cb, void *arg)
{
    return true;
};

lnSocket::status lnSocketQt::read(uint32_t &n, uint8_t **data)
{
    return lnSocket::Error;
}
/**
 */
lnSocket::status lnSocketQt::invoke(lnSocketEvent evt)
{
    return lnSocket::Error;
}
/**
 */
lnSocket::status lnSocketQt::flush()
{
    return lnSocket::Error;
}
/**
 */
lnSocket::status lnSocketQt::disconnectClient()
{
    return lnSocket::Error;
}
/**
 */
lnSocket::status lnSocketQt::asyncMode()
{
    return lnSocket::Error;
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
    return lnSocket::Error;
}
/**
 */
lnSocket::status lnSocketQt::write(uint32_t n, const uint8_t *data, uint32_t &done)
{
    return lnSocket::Error;
}
/**

lnSocket:: status lnSocketQt::read(uint32_t n, uint8_t *data, uint32_t &done) {
    return lnSocket::Error;
}

lnSocket::status lnSocketQt::close() {
    return lnSocket::Error;
}
*

*/
#if 0

/**
 * @class lnSocketQt
 * @brief [TODO:description]
 *
 */
/**
 * @brief [TODO:description]
 *
 * @param port [TODO:parameter]
 * @param cb [TODO:parameter]
 * @param arg [TODO:parameter]
 * @return [TODO:return]
 */
lnSocket *lnSocket::create(uint16_t port, lnSocketCb cb, void *arg)
{
    lnSocketQt *q = new lnSocketQt(port, NULL, NULL);
    if (!q->_server->startServer(port))
    {
        printf("!! Cannot start server\n");
        return NULL;
    }
    return q;
}
/**
 * @brief [TODO:description]
 *
 * @return [TODO:return]
 */
lnSocket::status lnSocketQt::accept()
{
    qInfo() << "Waiting for connection on port " << _port;
    if (!_server->waitForClient())
    {
        qDebug("Cannot listen\n");
        exit(-1);
    }
    return lnSocket::Ok;
}
/**
 * @brief [TODO:description]
 *
 * @param n [TODO:parameter]
 * @param data [TODO:parameter]
 * @param done [TODO:parameter]
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
 * @brief [TODO:description]
 *
 * @param n [TODO:parameter]
 * @param data [TODO:parameter]
 * @param done [TODO:parameter]
 */
lnSocket::status lnSocketQt::read(uint32_t n, uint8_t *data, uint32_t &done)
{
    int o = 0;
    QByteArray bytes = _server->readBytes(n);
    memcpy(data, bytes.data(), bytes.size());
    done = bytes.size();
    return lnSocket::Ok;
}
/**
 * @brief [TODO:description]
 *
 * @return [TODO:return]
 */
lnSocket::status lnSocketQt::flush()
{
    Q_ASSERT(0);
    return lnSocket::Ok;
}
/**
 * @brief [TODO:description]
 *
 * @return [TODO:return]
 */
lnSocket::status lnSocketQt::close()
{
    Q_ASSERT(0);
    return lnSocket::Ok;
}
//-----------------------------------------------------------
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
lnSocket *current_connection = NULL;

extern void exit_from_bmp();
static int x(uint8_t c)
{
    if (c >= '0' && c <= '9')
        return c - '0';
    if (c >= 'a' && c <= 'f')
        return 10 + c - 'a';
    if (c >= 'A' && c <= 'F')
        return 10 + c - 'A';
    return '.';
}
//
void cb(lnSocketEvent evt, void *arg)
{
    printf(" Socket callback , event = 0x%x\n", evt);
}

/*
 */
void initTcpLayer()
{
    current_connection = lnSocket::create(2000, cb, NULL);
}
/**
 * @brief [TODO:description]
 *
 * @param sz [TODO:parameter]
 * @param ptr [TODO:parameter]
 */
extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    uint32_t o = 0;
    if (!current_connection)
        return;
    while (sz)
    {
        if (current_connection->write(sz, ptr, o) == lnSocket::Ok)
        {
            ptr += o;
            sz -= o;
        }
        else
        {
            QBMPLOG("cannot write \n");
            exit(-1);
        }
    }
}
/**
 * @brief [TODO:description]
 */
extern "C" void rngdb_output_flush_c()
{
    if (current_connection)
    {
        QBMPLOG(" flush \n");
        current_connection->flush();
        current_connection->flush();
    }
    QBMPLOG("\n");
}
#endif

extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
}
extern "C" void rngdb_output_flush_c()
{
}
void initTcpLayer()
{
}
//
