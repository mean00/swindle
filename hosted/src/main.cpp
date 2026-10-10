/**
 * @file main.cpp
 * @brief Hosted-mode entry point: GDB server loop over TCP.
 */

#include <QCoreApplication>
#include <QDebug>
#include <QObject>
#include <QThread>
#include <QtGlobal>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "lnLWIP.h"
#include "hosted_constants.h"
#include "gdb_decoder.h"

extern "C"
{
#include "exception.h"
#include "gdb_if.h"
#include "gdb_main.h"
#include "gdb_packet.h"
#include "general.h"
#include "morse.h"
#include "target.h"
}

#define LN_ARCH LN_ARCH_ARM
#include "lnSocketRunner.h"

#define RUNNER_GDB_PORT DEFAULT_GDB_TCP_PORT
#define DEBUGME(...) do {} while (0)

// Intercept rngdb_send_data_c from bmp_net_gdb.h to decode replies on the fly
#define rngdb_send_data_c orig_rngdb_send_data_c
#include "../../swindle/src/net/bmp_net_gdb.h"
#undef rngdb_send_data_c

extern "C" void rngdb_send_data_c(uint32_t sz, const uint8_t *ptr)
{
    gdb_decode_outgoing(ptr, sz);
    orig_rngdb_send_data_c(sz, ptr);
}

extern void initTcpLayer();
extern "C" void platform_timeout_init();
extern "C" void platform_init(int argc, char **argv);

/**
 * @brief Custom Qt message handler that redirects messages to stderr.
 */
static void customHandler(QtMsgType type, const QMessageLogContext &context, const QString &msg)
{
    (void)type;
    (void)context;
    QByteArray localMsg = msg.toLocal8Bit();
    fprintf(stderr, "%s", localMsg.constData());
}

/**
 * @brief Exit application cleanly on BMP shutdown.
 */
extern "C" void exit_from_bmp()
{
    QCoreApplication::exit(0);
}

/**
 * @brief lwIP network event callback.
 */
static void sys_network(lnLwipEvent evt, void *arg)
{
    (void)arg;
    printf("[Network] lwIP system event: 0x%x\n", static_cast<unsigned int>(evt));
}

static lnFastEventGroup s_network_eventGroup;

/**
 * @brief Process socket events for a given runner.
 */
void process_sockets(socketRunner *runner, uint32_t global, uint32_t locl)
{
    uint32_t limited = (locl >> runner->shift()) & socketRunner::Mask;
    runner->process_events(limited | global);
}

/**
 * @brief Background thread hosting the GDB TCP server.
 */
class GdbServerThread : public QThread
{
  protected:
    void run() override
    {
        const char *argv[2] = {"", nullptr};
        platform_init(0, const_cast<char **>(argv));
        initTcpLayer();
        lnLWIP::start(sys_network, nullptr);

        s_network_eventGroup.takeOwnership();
        runnerGdb = new socketRunnerGdb(s_network_eventGroup, 0);
        runnerGdb->sendEvent(socketRunner::Up);

        uint32_t mask = 0xffffffffUL;
        mask &= ~(socketRunner::CanWrite << 0);
        const uint32_t global_mask = (socketRunner::Up | socketRunner::Down);

        printf("[GDB Server] Listening on localhost:%d\n", DEFAULT_GDB_TCP_PORT);

        while (true)
        {
            uint32_t events = s_network_eventGroup.waitEvents(mask, EVENT_POLL_TIMEOUT_MS);
            uint32_t global_events = events & global_mask;
            uint32_t local_events = events & (~global_mask);
            process_sockets(runnerGdb, global_events, local_events);
        }
    }
};

/**
 * @brief Hook into GDB packet reception to decode commands on the fly.
 */
static void hosted_rngdbstub_run_intercept(uint32_t len, const uint8_t *data)
{
    gdb_decode_incoming(data, len);
    rngdbstub_run(len, data);
}

// Redirect socketRunnerGdb::process_incoming_data() to the decoding interceptor
#define rngdbstub_run hosted_rngdbstub_run_intercept
#include "bmp_net_gdb.cpp"
#undef rngdbstub_run

static constexpr size_t TEMP_BUFFER_WORDS = 1024;
static uint32_t s_temp_buffer[TEMP_BUFFER_WORDS];

extern "C" uint8_t *get_temp_buffer_c(uint32_t size)
{
    if (size >= sizeof(s_temp_buffer))
    {
        fprintf(stderr, "Buffer request size %u exceeds %zu\n", size, sizeof(s_temp_buffer));
        exit(1);
    }
    return reinterpret_cast<uint8_t *>(s_temp_buffer);
}

/**
 * @brief Application entry point.
 */
int main(int argc, char **argv)
{
    // Check command line arguments and environment for GDB decoding flag
    for (int i = 1; i < argc; ++i)
    {
        if (strcmp(argv[i], "--decode-gdb") == 0 || strcmp(argv[i], "-d") == 0 || strcmp(argv[i], "-g") == 0)
        {
            gdb_decoder_set_enabled(true);
        }
        else if (strcmp(argv[i], "--help") == 0 || strcmp(argv[i], "-h") == 0)
        {
            printf("Usage: swindle_hosted [options]\n");
            printf("Options:\n");
            printf("  --decode-gdb, -d, -g  Decode GDB commands on the fly in human-readable format\n");
            printf("  --help, -h            Show this help message\n");
            return 0;
        }
    }

    if (getenv("SWINDLE_DECODE_GDB") != nullptr)
    {
        gdb_decoder_set_enabled(true);
    }

    qWarning() << "======================";
    qWarning() << "*  Swindle Hosted    *";
    qWarning() << "======================";

    QCoreApplication app(argc, argv);
    qInstallMessageHandler(customHandler);

    platform_timeout_init();

    GdbServerThread *thread = new GdbServerThread;
    thread->start();

    return app.exec();
}
