/**
 * @file esp_socket_rust.cpp
 * @brief Qt services the hosted Rust SocketRunner needs, behind a C ABI.
 *
 * Qt/Qt-network work that cannot be done on the Rust side:
 *  - the local listening socket (`lnSocket`/`lnSocketQt`, see esp_qtnetwork),
 *  - the TCP link to a network probe (the `swindle_hosted_net` relay),
 *  - the `lnfast_event_group_*` primitives `rs_rtos` binds to, implemented over
 *    the Qt-backed `lnFastEventGroup` class of esp_eventgroup.cpp.
 */

#include "esp_socket_rust.h"

#include <QAbstractSocket>
#include <QByteArray>
#include <QString>
#include <QTcpSocket>
#include <cstdio>
#include <cstring>

#include "esp_qtnetwork.h"
#include "hosted_constants.h"
#include "lnFreeRTOS_pp.h"
#include "lnLWIP.h"
// The opaque C view of lnFastEventGroup, i.e. the type rs_rtos passes around.
extern "C"
{
#include "lnFast_EventGroup_c.h"
}

/* ------------------------------------------------------------------------- */
/*  The local listening socket                                               */
/* ------------------------------------------------------------------------- */

/**
 * Forwards the Qt socket event to the Rust listener, unchanged.
 *
 * `lnSocketCb` has a single cookie, and the Rust side needs two things in it (its
 * thunk and the listener), so they are kept here: a hosted executable serves one
 * GDB client socket, which is the same assumption `hosted_runner_new()` makes.
 */
static ln_socket_event_cb s_event_cb = nullptr;
static void *s_event_cookie = nullptr;

static void rust_socket_event(lnSocketEvent evt, void *)
{
    if (s_event_cb != nullptr)
    {
        s_event_cb(static_cast<uint32_t>(evt), s_event_cookie);
    }
}

extern "C" void *ln_socket_open(uint16_t port, ln_socket_event_cb cb, void *cookie)
{
    s_event_cb = cb;
    s_event_cookie = cookie;
    return lnSocket::create(port, rust_socket_event, nullptr);
}

#define SOCK(x) reinterpret_cast<lnSocket *>(x)

// Client bytes read from Qt but not yet handed to the Rust runner: one Qt read
// can return more than the caller's buffer holds.
static QByteArray s_pending;
static int s_pending_offset = 0;

extern "C" int ln_socket_read(void *sock, uint8_t *dst, uint32_t cap, uint32_t *n)
{
    if (sock == nullptr || dst == nullptr || cap == 0)
    {
        return 1;
    }
    // Hand out what is left of the previous read first: the Qt side returns one
    // QByteArray with everything queued, which can be larger than the caller's
    // buffer.
    if (s_pending_offset >= s_pending.size())
    {
        uint32_t len = 0;
        uint8_t *data = nullptr;
        if (SOCK(sock)->read(len, &data) != lnSocket::Ok || data == nullptr)
        {
            return 1;
        }
        s_pending = QByteArray(reinterpret_cast<const char *>(data), static_cast<int>(len));
        s_pending_offset = 0;
        if (s_pending.isEmpty())
        {
            *n = 0;
            return 0;
        }
        SOCK(sock)->freeReadData();
    }
    const int left = s_pending.size() - s_pending_offset;
    const int take = qMin(left, static_cast<int>(cap));
    memcpy(dst, s_pending.constData() + s_pending_offset, static_cast<size_t>(take));
    s_pending_offset += take;
    *n = static_cast<uint32_t>(take);
    return 0;
}

extern "C" int ln_socket_write(void *sock, uint32_t n, const uint8_t *data, uint32_t *done)
{
    if (sock == nullptr)
    {
        return 1;
    }
    return SOCK(sock)->write(n, data, *done) == lnSocket::Ok ? 0 : 1;
}

extern "C" int ln_socket_flush(void *sock)
{
    return (sock != nullptr && SOCK(sock)->flush() == lnSocket::Ok) ? 0 : 1;
}

extern "C" int ln_socket_write_buffer_available(void *sock, uint32_t *n)
{
    if (sock == nullptr)
    {
        return 1;
    }
    return SOCK(sock)->writeBufferAvailable(*n) == lnSocket::Ok ? 0 : 1;
}

extern "C" int ln_socket_disconnect_client(void *sock)
{
    return (sock != nullptr && SOCK(sock)->disconnectClient() == lnSocket::Ok) ? 0 : 1;
}

extern "C" void ln_socket_close(void *sock)
{
    delete SOCK(sock);
}

/* ------------------------------------------------------------------------- */
/*  The link to a network probe                                              */
/* ------------------------------------------------------------------------- */

namespace
{
/**
 * @brief One TCP link to the probe, held for the duration of a GDB session.
 *
 * Same rules as the serial back end: the socket is created on the thread that
 * uses it (the GDB server thread), and a write only waits for the flush when Qt
 * actually had to buffer.
 */
class ProbeTcpLink
{
  public:
    ~ProbeTcpLink()
    {
        close();
    }

    bool open(const QString &host, quint16 port)
    {
        close();
        _socket = new QTcpSocket();
        _socket->setSocketOption(QAbstractSocket::LowDelayOption, 1);
        _socket->connectToHost(host, port);
        if (!_socket->waitForConnected(PROBE_CONNECT_TIMEOUT_MS))
        {
            fprintf(stderr, "[Net] Cannot reach the probe at %s:%u : %s\n", host.toLocal8Bit().constData(),
                    static_cast<unsigned>(port), _socket->errorString().toLocal8Bit().constData());
            close();
            return false;
        }
        printf("[Net] Connected to the probe at %s:%u\n", host.toLocal8Bit().constData(), static_cast<unsigned>(port));
        fflush(stdout);
        return true;
    }

    bool isOpen() const
    {
        return _socket != nullptr && _socket->state() == QAbstractSocket::ConnectedState;
    }

    /** What the probe sent, waiting at most `waitMs` for the first byte. */
    int read(int waitMs, uint8_t *dst, int cap)
    {
        if (_socket == nullptr)
        {
            return 0;
        }
        if (waitMs > 0 && _socket->bytesAvailable() == 0)
        {
            _socket->waitForReadyRead(waitMs);
        }
        const QByteArray chunk = _socket->read(cap);
        const int n = chunk.size();
        if (n > 0)
        {
            memcpy(dst, chunk.constData(), static_cast<size_t>(n));
        }
        return n;
    }

    bool write(const uint8_t *data, int size)
    {
        if (!isOpen())
        {
            return false;
        }
        int remaining = size;
        const char *ptr = reinterpret_cast<const char *>(data);
        while (remaining > 0)
        {
            const qint64 written = _socket->write(ptr, remaining);
            if (written <= 0)
            {
                fprintf(stderr, "[Net] Write to the probe failed: %s\n", _socket->errorString().toLocal8Bit().constData());
                return false;
            }
            remaining -= static_cast<int>(written);
            ptr += written;
        }
        _socket->flush();
        if (_socket->bytesToWrite() > 0)
        {
            _socket->waitForBytesWritten(SERIAL_WRITE_DRAIN_TIMEOUT_MS);
        }
        return true;
    }

    void close()
    {
        if (_socket != nullptr)
        {
            _socket->abort();
            delete _socket;
            _socket = nullptr;
        }
    }

  private:
    QTcpSocket *_socket{nullptr};
};
} // namespace

#define LINK(x) static_cast<ProbeTcpLink *>(x)

extern "C" void *probe_link_open(const char *host, uint16_t port)
{
    if (host == nullptr)
    {
        return nullptr;
    }
    auto *link = new ProbeTcpLink();
    if (!link->open(QString::fromLocal8Bit(host), port))
    {
        delete link;
        return nullptr;
    }
    return link;
}

extern "C" bool probe_link_is_open(void *link)
{
    return link != nullptr && LINK(link)->isOpen();
}

extern "C" int probe_link_read(void *link, int32_t wait_ms, uint8_t *dst, int32_t cap)
{
    if (link == nullptr || dst == nullptr || cap <= 0)
    {
        return 0;
    }
    return LINK(link)->read(wait_ms, dst, cap);
}

extern "C" bool probe_link_write(void *link, const uint8_t *data, int32_t size)
{
    return link != nullptr && LINK(link)->write(data, size);
}

extern "C" void probe_link_close(void *link)
{
    delete LINK(link);
}

/* ------------------------------------------------------------------------- */
/*  Packet observers (C++ only, so wrapped for the Rust side)                */
/* ------------------------------------------------------------------------- */

#include "gdb_decoder.h"

extern "C" void gdb_decode_incoming_c(const uint8_t *data, uint32_t len)
{
    gdb_decode_incoming(data, len);
}

extern "C" void gdb_decode_outgoing_c(const uint8_t *data, uint32_t len)
{
    gdb_decode_outgoing(data, len);
}

/* ------------------------------------------------------------------------- */
/*  Event groups for rs_rtos                                                 */
/*                                                                           */
/*  The object is the Qt-backed lnFastEventGroup of esp_eventgroup.cpp; this  */
/*  is the same C view the probe build gets from lnFast_EventGroup_c.cpp.     */
/* ------------------------------------------------------------------------- */

#define EVG(x) reinterpret_cast<lnFastEventGroup *>(const_cast<lnfast_event_group_c *>(x))

extern "C" lnfast_event_group_c *lnfast_event_group_create()
{
    return reinterpret_cast<lnfast_event_group_c *>(new lnFastEventGroup());
}

extern "C" void lnfast_event_group_delete(const lnfast_event_group_c *inst)
{
    delete EVG(inst);
}

extern "C" void lnfast_event_group_takeOwnership(const lnfast_event_group_c *inst)
{
    EVG(inst)->takeOwnership();
}

extern "C" void lnfast_event_group_set_events(const lnfast_event_group_c *inst, uint32_t event)
{
    EVG(inst)->setEvents(event);
}

extern "C" uint32_t lnfast_event_group_wait_events(const lnfast_event_group_c *inst, uint32_t mask, int32_t timeout)
{
    return EVG(inst)->waitEvents(mask, timeout);
}

extern "C" uint32_t lnfast_event_group_read_events(const lnfast_event_group_c *inst, uint32_t mask)
{
    return EVG(inst)->readEvents(mask);
}
