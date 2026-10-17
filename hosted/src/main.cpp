/**
 * @file main.cpp
 * @brief Hosted-mode entry point: GDB server loop over TCP (CLI version).
 */

#include <QCoreApplication>
#include <QDebug>
#include <QObject>
#include <QSocketNotifier>
#include <QtGlobal>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <termios.h>
#include <unistd.h>

#include "gdb_server_thread.h"

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

static struct termios s_orig_termios;
static bool s_termios_saved = false;

static void restore_terminal()
{
    if (s_termios_saved)
    {
        tcsetattr(STDIN_FILENO, TCSANOW, &s_orig_termios);
        s_termios_saved = false;
    }
}

static void setup_terminal()
{
    if (isatty(STDIN_FILENO))
    {
        if (tcgetattr(STDIN_FILENO, &s_orig_termios) == 0)
        {
            s_termios_saved = true;
            atexit(restore_terminal);

            struct termios raw = s_orig_termios;
            raw.c_lflag &= ~(ICANON | ECHO);
            raw.c_cc[VMIN] = 1;
            raw.c_cc[VTIME] = 0;
            tcsetattr(STDIN_FILENO, TCSANOW, &raw);
        }
    }
}

static void signal_handler(int sig)
{
    restore_terminal();
    _exit(128 + sig);
}

/**
 * @brief Exit application cleanly on BMP shutdown.
 */
extern "C" void exit_from_bmp()
{
    restore_terminal();
    QCoreApplication::exit(0);
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

    setup_terminal();
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);

    platform_timeout_init();

    GdbServerThread *thread = new GdbServerThread;
    thread->start();

    QSocketNotifier stdin_notifier(STDIN_FILENO, QSocketNotifier::Read);
    QObject::connect(&stdin_notifier, &QSocketNotifier::activated, [&]() {
        char ch = 0;
        ssize_t n = ::read(STDIN_FILENO, &ch, 1);
        if (n <= 0)
        {
            stdin_notifier.setEnabled(false);
            return;
        }
        if (ch == 'q' || ch == 'Q')
        {
            printf("\n[Host] 'q' pressed, exiting cleanly...\n");
            fflush(stdout);
            thread->stopServer();
            thread->wait(1000);
            restore_terminal();
            QCoreApplication::quit();
        }
    });

    printf("[Host] Press 'q' to quit.\n");
    fflush(stdout);

    int ret = app.exec();

    if (thread->isRunning())
    {
        thread->stopServer();
        thread->wait(1000);
    }
    delete thread;
    restore_terminal();

    return ret;
}
