/**
 * @file bmp_qttcp.cpp
 * @brief Network probe back end for swindle_hosted_net.
 *
 * A network probe (the W5500 / lwIP build) answers the GDB remote serial
 * protocol on its own TCP port, so this back end needs neither the BMP core nor
 * the RPC: it relays the session bytes verbatim between the local GDB client
 * socket and the probe socket.
 *
 * Everything else — the GDB server on the local port, the event loop, the
 * packet decoders — is the same code the USB build uses (see hosted_probe.h).
 */

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include <QAbstractSocket>
#include <QByteArray>
#include <QString>
#include <QTcpSocket>

#include "esp_socket_rust.h"
#include "hosted_constants.h"
#include "hosted_probe.h"

/** Option / environment names, kept next to their use. */
static constexpr const char *OPTION_HOST = "--host";
static constexpr const char *OPTION_PORT = "--port";
static constexpr const char *ENV_PROBE = "SWINDLE_NET_PROBE";

namespace
{

/** Where the probe waits for a GDB client. */
struct ProbeEndpoint
{
    QString host;
    quint16 port{DEFAULT_GDB_TCP_PORT};
};

/** The endpoint chosen on the command line, read by the relay. */
ProbeEndpoint s_endpoint;

/** Parse a decimal TCP port, rejecting anything that is not one. */
bool parse_port(const char *text, quint16 &port)
{
    if (text == nullptr || text[0] == '\0')
    {
        return false;
    }
    char *end = nullptr;
    const long value = strtol(text, &end, 10);
    if (end == text || *end != '\0' || value <= 0 || value > 0xffff)
    {
        return false;
    }
    port = static_cast<quint16>(value);
    return true;
}

/** Parse `host` or `host:port` into @p endpoint (the port part is optional). */
bool parse_endpoint(const char *text, ProbeEndpoint &endpoint)
{
    if (text == nullptr)
    {
        return false;
    }
    QString value = QString::fromLocal8Bit(text).trimmed();
    if (value.isEmpty())
    {
        return false;
    }
    const int colon = value.lastIndexOf(':');
    if (colon >= 0)
    {
        const QByteArray digits = value.mid(colon + 1).toLatin1();
        quint16 port = endpoint.port;
        if (!parse_port(digits.constData(), port))
        {
            return false;
        }
        endpoint.port = port;
        value = value.left(colon);
    }
    if (value.isEmpty())
    {
        return false;
    }
    endpoint.host = value;
    return true;
}
} // namespace

/**
 * @brief Build the GDB runner for a network probe.
 *
 * The endpoint is passed to the Rust runner (`RelayHandler` in
 * swindle/rs/rs_swindle/src/hosted/qt_runner.rs), which owns the link and relays
 * the session; this file only parses where to connect.
 */
void *hosted_create_runner(lnFastEventGroup &eventGroup)
{
    return hosted_runner_new(&eventGroup, DEFAULT_GDB_TCP_PORT, s_endpoint.host.toLocal8Bit().constData(),
                             s_endpoint.port);
}

void hosted_probe_usage()
{
    printf("Network probe options (probe endpoint):\n");
    printf("  --host <name> --port <n>  probe GDB server reachable over TCP\n");
    printf("  <host>[:<port>]           same, as a positional argument\n");
    printf("  %s=host[:port]            same, from the environment\n", ENV_PROBE);
    printf("  The probe port defaults to %d (its GDB server); this host keeps listening on %d.\n",
           DEFAULT_GDB_TCP_PORT, DEFAULT_GDB_TCP_PORT);
}

bool hosted_probe_parse_args(int argc, char **argv)
{
    // Environment first, the command line wins over it.
    const char *from_env = getenv(ENV_PROBE);
    if (from_env != nullptr && from_env[0] != '\0' && !parse_endpoint(from_env, s_endpoint))
    {
        fprintf(stderr, "[Net] Ignoring malformed %s=%s\n", ENV_PROBE, from_env);
    }

    for (int i = 1; i < argc; ++i)
    {
        if (strcmp(argv[i], OPTION_HOST) == 0)
        {
            if (++i >= argc)
            {
                fprintf(stderr, "[Net] %s needs a host name\n", OPTION_HOST);
                return false;
            }
            s_endpoint.host = QString::fromLocal8Bit(argv[i]);
        }
        else if (strcmp(argv[i], OPTION_PORT) == 0)
        {
            if (++i >= argc || !parse_port(argv[i], s_endpoint.port))
            {
                fprintf(stderr, "[Net] %s needs a port number\n", OPTION_PORT);
                return false;
            }
        }
        else if (argv[i][0] != '-')
        {
            // `<host> <port>` is accepted as well as `<host>[:<port>]`.
            quint16 port = s_endpoint.port;
            if (!s_endpoint.host.isEmpty() && parse_port(argv[i], port))
            {
                s_endpoint.port = port;
            }
            else if (!parse_endpoint(argv[i], s_endpoint))
            {
                fprintf(stderr, "[Net] Cannot parse the probe address '%s'\n", argv[i]);
                return false;
            }
        }
    }

    if (s_endpoint.host.isEmpty())
    {
        fprintf(stderr, "[Net] No probe given: --host <name> [--port <n>], <host>[:<port>] or %s\n", ENV_PROBE);
        return false;
    }
    return true;
}

void hosted_probe_print_banner(bool raw_dmi)
{
    // No BMP core in this build: how memory is fetched is the probe's business.
    (void)raw_dmi;
    printf("[Host] Relaying GDB to the probe at %s:%u\n", s_endpoint.host.toLocal8Bit().constData(),
           static_cast<unsigned>(s_endpoint.port));
    fflush(stdout);
}

/**
 * @brief Probe back end initialisation (called from the GDB server thread).
 *
 * The endpoint was validated by hosted_probe_parse_args(); the link itself is
 * opened per GDB session so the probe keeps its single GDB slot free when no
 * debugger is attached.
 */
extern "C" void platform_init(int argc, char **argv)
{
    (void)argc;
    (void)argv;
}

