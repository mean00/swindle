/**
 * @file bmp_qtserial.cpp
 * @brief Qt serial port transport implementation for swindle_hosted.
 */

#include <cstdarg>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <atomic>
#include <chrono>
#include <poll.h>
#include <unistd.h>
#include <vector>

extern "C"
{
#include "platform_support.h"
}

#include <QDateTime>
#include <QFile>
#include <QSerialPort>
#include <QSerialPortInfo>

#include "bmp_logger.h"
#include "hosted_constants.h"

/**
 * @brief Probe selected for the session.
 *
 * All the transport needs is the serial port its GDB CDC is reachable on, so
 * the blackmagic probe description is reduced to that one field.
 */
typedef struct bmda_probe
{
    char port[512];
} bmda_probe_s;

#define ASSERT_OR_EXIT(condition)                                                                                      \
    if (!(condition))                                                                                                  \
    {                                                                                                                  \
        fprintf(stderr, "Assertion failed: %s at %s:%d\n", #condition, __FILE__, __LINE__);                            \
        exit(1);                                                                                                       \
    }

static QSerialPort *s_serial_port = nullptr;

extern "C" void platform_write_flush();
extern "C" int platform_buffer_write_buffered(const uint8_t *data, int size);
extern "C" int platform_buffer_read(uint8_t *data, int maxsize);
extern "C" int platform_buffer_write(const uint8_t *data, int size);
void decoderRequest(int size, const uint8_t *data);
void decoderReply(int size, const uint8_t *data);

// Keyboard hand-off with the CLI's 'q' handler (see ask_probe_choice()). The
// prompt lives here rather than in main.cpp because this file is part of both
// executables (CLI and Qt), while main.cpp is only the CLI one.
static std::atomic<bool> s_stdin_prompt_active{false};
static std::atomic<int> s_stdin_prompt_char{-1};

/** True while a foreground prompt owns the keyboard. */
extern "C" bool hosted_stdin_prompt_is_active()
{
    return s_stdin_prompt_active.load();
}

/** Hand a character read by the caller's key handler to the active prompt. */
extern "C" void hosted_stdin_prompt_push(int ch)
{
    s_stdin_prompt_char.store(ch);
}

/** Take the character handed over by the key handler, -1 when none. */
static int hosted_stdin_prompt_take()
{
    return s_stdin_prompt_char.exchange(-1);
}

/** Start / end the keyboard hand-off. */
static void hosted_stdin_prompt_begin()
{
    s_stdin_prompt_char.store(-1);
    s_stdin_prompt_active.store(true);
}

static void hosted_stdin_prompt_end()
{
    s_stdin_prompt_active.store(false);
    s_stdin_prompt_char.store(-1);
}

// Outgoing write buffer
static int s_platform_write_index = 0;
static uint8_t s_platform_buffer[PLATFORM_BUFFER_SIZE];

// Incoming serial ring buffer
static uint8_t s_serial_ring_buffer[SERIAL_BUFFER_SIZE];
static int s_serial_head = 0;
static int s_serial_tail = 0;

enum class SerialParserState
{
    WAIT_FRAME_START,
    READ_PAYLOAD,
    FRAME_DONE
};

/**
 * @brief Read a single character from the serial ring buffer, refilling from QSerialPort as needed.
 */
static bool readChar(uint8_t *data, int wait_ms = 100)
{
    if (s_serial_head != s_serial_tail)
    {
        *data = s_serial_ring_buffer[s_serial_head++];
        return true;
    }

    if (!s_serial_port || !s_serial_port->isOpen())
    {
        return false;
    }

    s_serial_head = s_serial_tail = 0;

    int space = SERIAL_BUFFER_SIZE;
    int available = s_serial_port->bytesAvailable();
    if (available > 0)
    {
        int to_read = (available > space) ? space : available;
        int bytes_read = s_serial_port->read(reinterpret_cast<char *>(s_serial_ring_buffer), to_read);
        if (bytes_read > 0)
        {
            s_serial_tail = bytes_read;
            *data = s_serial_ring_buffer[s_serial_head++];
            return true;
        }
        if (bytes_read < 0)
        {
            fprintf(stderr, "[Serial] Read error encountered on port\n");
            return false;
        }
    }

    if (s_serial_port->waitForReadyRead(wait_ms))
    {
        available = s_serial_port->bytesAvailable();
        int to_read = (available > space) ? space : available;
        int bytes_read = s_serial_port->read(reinterpret_cast<char *>(s_serial_ring_buffer), to_read);
        if (bytes_read > 0)
        {
            s_serial_tail = bytes_read;
            *data = s_serial_ring_buffer[s_serial_head++];
            return true;
        }
        if (bytes_read < 0)
        {
            fprintf(stderr, "[Serial] Read error encountered on port\n");
            return false;
        }
    }

    return false;
}

/**
 * @brief Write data into the platform buffer; flushes when buffer fills.
 */
extern "C" int platform_buffer_write_buffered(const uint8_t *data, int size)
{
    if (size + s_platform_write_index >= PLATFORM_BUFFER_SIZE)
    {
        platform_write_flush();
    }

    // A chunk that cannot fit the buffer even when empty (a whole RV.MEM_WRITE block)
    // goes straight to the port; the flush above kept the byte order.
    if (size >= PLATFORM_BUFFER_SIZE)
    {
        platform_buffer_write(data, size);
        return size;
    }

    ASSERT_OR_EXIT(size + s_platform_write_index < PLATFORM_BUFFER_SIZE);
    memcpy(s_platform_buffer + s_platform_write_index, data, size);
    s_platform_write_index += size;
    return size;
}

/**
 * @brief Flush the platform write buffer to the serial port.
 */
extern "C" void platform_write_flush()
{
    if (s_platform_write_index == 0)
    {
        return;
    }
    platform_buffer_write(s_platform_buffer, s_platform_write_index);
    s_platform_write_index = 0;
}

/**
 * @brief Read a RPC response payload from serial with framing delimiter: '&' <payload> '#'.
 */
extern "C" int platform_buffer_read(uint8_t *data, int maxsize)
{
    SerialParserState state = SerialParserState::WAIT_FRAME_START;
    int payload_len = 0;
    auto start_time = std::chrono::steady_clock::now();
    const auto timeout = std::chrono::milliseconds(2000);

    while (true)
    {
        uint8_t c = 0;
        if (!readChar(&c, 100))
        {
            if (std::chrono::steady_clock::now() - start_time > timeout)
            {
                fprintf(stderr, "[Serial] Timeout waiting for RPC frame reply (state: %d, bytes: %d)\n",
                        static_cast<int>(state), payload_len);
                return SERIAL_READ_ERROR_IO;
            }
            continue;
        }

        switch (state)
        {
        case SerialParserState::WAIT_FRAME_START:
            if (c == FRAME_START_DELIMITER)
            {
                state = SerialParserState::READ_PAYLOAD;
            }
            break;

        case SerialParserState::READ_PAYLOAD:
            if (c == FRAME_END_DELIMITER)
            {
                data[payload_len] = '\0';
                decoderReply(payload_len, data);
                return payload_len;
            }

            data[payload_len++] = c;
            if (payload_len >= maxsize)
            {
                fprintf(stderr, "[Serial] Buffer overflow: payload exceeds %d bytes\n", maxsize);
                return SERIAL_READ_ERROR_OVERFLOW;
            }
            break;

        case SerialParserState::FRAME_DONE:
        default:
            return SERIAL_READ_ERROR_IO;
        }
    }
}

/**
 * @brief Write raw unbuffered bytes directly to the probe serial port.
 */
extern "C" int platform_buffer_write(const uint8_t *data, int size)
{
    if (!s_serial_port || !s_serial_port->isOpen())
    {
        return size;
    }

    decoderRequest(size, data);

    int remaining = size;
    const char *ptr = reinterpret_cast<const char *>(data);

    while (remaining > 0)
    {
        qint64 written = s_serial_port->write(ptr, remaining);
        if (written <= 0)
        {
            fprintf(stderr, "[Serial] Write error occurred, aborting.\n");
            exit(1);
        }
        remaining -= static_cast<int>(written);
        ptr += written;
    }

    s_serial_port->flush();
    // QSerialPort::write() above already put the bytes on the wire; only when it
    // had to buffer a partial write (bytesToWrite() > 0) is a drain needed.
    // Calling waitForBytesWritten() unconditionally costs the full timeout per
    // RPC because the write notification never fires once the buffer is empty.
    if (s_serial_port->bytesToWrite() > 0)
    {
        s_serial_port->waitForBytesWritten(SERIAL_WRITE_DRAIN_TIMEOUT_MS);
    }
    return 0;
}

/** Time the operator has to answer the probe-selection prompt, in seconds. */
static constexpr int PROBE_PROMPT_TIMEOUT_S = 30;
/** How often the prompt looks for a typed character, in milliseconds. */
static constexpr int PROBE_PROMPT_POLL_MS = 50;

/** One supported probe found on the USB serial ports. */
struct HostedProbe
{
    QString port; // system location, e.g. /dev/ttyACM0
    quint16 vid;
    quint16 pid;
};

/**
 * @brief Is @p portName the GDB CDC of a probe (its first CDC)?
 *
 * A Swindle board exposes several CDC-ACM ports — GDB, UART bridge, logger —
 * which all carry the same VID/PID, so an unfiltered list would offer ports no
 * GDB session can use. On Linux they differ by `bInterfaceNumber` and the GDB one
 * is the first. Where that file cannot be read the port is kept, so the list
 * degrades to "all of them" instead of "none".
 */
static bool is_gdb_cdc_port(const QString &portName)
{
    QFile number(QStringLiteral("/sys/class/tty/%1/device/bInterfaceNumber").arg(portName));
    if (!number.open(QIODevice::ReadOnly))
    {
        return true;
    }
    return number.readAll().trimmed() == QByteArrayLiteral("00");
}

/** Model string of a supported probe PID. */
static const char *probe_model_name(quint16 pid)
{
    switch (pid)
    {
    case USB_PID_SWINDLE_LN:
        return "Swindle GD32/CH32 Debugger";
    case USB_PID_SWINDLE_PICO:
        return "pSwindle RP2040 Pico Debugger";
    case USB_PID_BMP_CLASSIC:
        return "Classic Black Magic Probe";
    default:
        return "OpenMoko / BMP Debugger";
    }
}

/** Fill the probe the rest of the startup uses. */
static void select_probe(bmda_probe_s *info, const char *port)
{
    memset(info, 0, sizeof(*info));
    strncpy(info->port, port, sizeof(info->port) - 1);
}

/**
 * @brief Read one character from the terminal, waiting at most @p timeout_ms.
 * @return the character, or -1 on timeout / end of input.
 */
static int read_stdin_char(int timeout_ms)
{
    struct pollfd pfd = {STDIN_FILENO, POLLIN, 0};
    if (poll(&pfd, 1, timeout_ms) <= 0)
    {
        return -1;
    }
    uint8_t c = 0;
    if (read(STDIN_FILENO, &c, 1) != 1)
    {
        return -1;
    }
    return static_cast<int>(c);
}

/**
 * @brief Ask the operator which of the probes to use.
 *
 * The terminal is in raw mode (`main.cpp` clears ICANON and ECHO so a single
 * 'q' can quit the app), so the number is assembled and echoed here. The 'q'
 * handler is suspended meanwhile — `hosted_stdin_prompt_begin/end()` — and any
 * character it still receives is handed back through
 * `hosted_stdin_prompt_take()` instead of being swallowed.
 *
 * @param count number of probes to choose from (>= 1)
 * @return 1-based index of the chosen probe; 1 when nothing usable was entered
 */
static size_t ask_probe_choice(size_t count)
{
    if (!isatty(STDIN_FILENO))
    {
        fprintf(stderr, "[Serial] stdin is not a terminal: using probe 1 (set BMP_DEVICE to pick another)\n");
        return 1;
    }

    printf("\nSelect the probe to use [1-%zu, enter for 1]: ", count);
    fflush(stdout);

    hosted_stdin_prompt_begin();
    size_t value = 0;
    bool have_digit = false;
    bool answered = false;
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(PROBE_PROMPT_TIMEOUT_S);
    while (!answered && std::chrono::steady_clock::now() < deadline)
    {
        int ch = hosted_stdin_prompt_take();
        if (ch < 0)
        {
            ch = read_stdin_char(PROBE_PROMPT_POLL_MS);
        }
        if (ch < 0)
        {
            continue;
        }
        if (ch == '\r' || ch == '\n')
        {
            answered = true;
        }
        else if (ch >= '0' && ch <= '9')
        {
            value = value * 10 + static_cast<size_t>(ch - '0');
            have_digit = true;
            putchar(ch);
            fflush(stdout);
        }
    }
    hosted_stdin_prompt_end();
    printf("\n");

    if (!have_digit || value < 1 || value > count)
    {
        printf("[Serial] No valid choice: using probe 1\n");
        return 1;
    }
    return value;
}

/**
 * @brief Find and enumerate compatible BMP/Swindle probes connected via USB serial.
 *
 * Several probes are normal on a bench (an LN probe and a pico, say), and the
 * port order is not stable, so the choice is explicit: `BMP_DEVICE=<port>` when
 * set, a prompt when more than one probe is present and the operator has a
 * terminal, probe 1 otherwise.
 */
static bool find_debuggers(bmda_probe_s *info)
{
    std::vector<HostedProbe> probes;
    for (const QSerialPortInfo &portInfo : QSerialPortInfo::availablePorts())
    {
        if (!portInfo.hasVendorIdentifier() || !portInfo.hasProductIdentifier())
        {
            continue;
        }

        const quint16 vid = portInfo.vendorIdentifier();
        const quint16 pid = portInfo.productIdentifier();
        const bool is_supported_probe = (pid == USB_PID_SWINDLE_LN ||
                                         pid == USB_PID_SWINDLE_PICO ||
                                         pid == USB_PID_BMP_CLASSIC);
        if (vid != USB_VID_OPENMOKO || !is_supported_probe)
        {
            continue;
        }
        // Only the GDB CDC answers GDB: skip the UART bridge and the logger.
        if (!is_gdb_cdc_port(portInfo.portName()))
        {
            continue;
        }

        probes.push_back({portInfo.systemLocation(), vid, pid});
    }

    // Non-interactive override: BMP_DEVICE=/dev/ttySwindleGdb2 (a udev symlink)
    // or /dev/ttyACM0. Checked before the enumeration so it also works for a port
    // QtSerialPort does not list.
    const char *requested = getenv("BMP_DEVICE");
    if (requested != nullptr && requested[0] != '\0')
    {
        if (access(requested, R_OK | W_OK) == 0)
        {
            select_probe(info, requested);
            printf("[Serial] Using probe: %s (BMP_DEVICE override)\n", requested);
            return true;
        }
        fprintf(stderr, "[Serial] BMP_DEVICE=%s is not usable: falling back to the probe list\n", requested);
    }

    if (probes.empty())
    {
        fprintf(stderr, "[Serial] No compatible Black Magic / Swindle probe found.\n");
        return false;
    }

    printf("Found %zu compatible probe(s):\n", probes.size());
    for (size_t i = 0; i < probes.size(); ++i)
    {
        printf("  %zu) %s  VID=0x%04x PID=0x%04x  %s\n", i + 1,
               probes[i].port.toLatin1().constData(), probes[i].vid, probes[i].pid,
               probe_model_name(probes[i].pid));
    }

    size_t chosen = 0;
    if (probes.size() > 1)
    {
        chosen = ask_probe_choice(probes.size()) - 1;
    }

    select_probe(info, probes[chosen].port.toLatin1().constData());
    printf("[Serial] Using probe: %s  VID=0x%04x PID=0x%04x  Model: %s\n",
           probes[chosen].port.toLatin1().constData(), probes[chosen].vid, probes[chosen].pid,
           probe_model_name(probes[chosen].pid));
    return true;
}

/**
 * @brief Create the serial port instance for the probe.
 */
static void bmp_ident(bmda_probe_s *info)
{
    printf("[Serial] Selecting port: %s\n", info->port);
    s_serial_port = new QSerialPort(QString(info->port));
}

/**
 * @brief Open and configure the serial port for 8N1 communication.
 */
static bool serial_open(void)
{
    if (!s_serial_port)
    {
        return false;
    }

    if (!s_serial_port->open(QIODevice::ReadWrite))
    {
        fprintf(stderr, "[Serial] Failed to open port %s: %s\n",
                s_serial_port->portName().toLocal8Bit().constData(),
                s_serial_port->errorString().toLocal8Bit().constData());
        return false;
    }

    s_serial_port->setBaudRate(SERIAL_BAUD_RATE);
    s_serial_port->setDataBits(QSerialPort::Data8);
    s_serial_port->setParity(QSerialPort::NoParity);
    s_serial_port->setStopBits(QSerialPort::OneStop);
    s_serial_port->setFlowControl(QSerialPort::NoFlowControl);

    printf("[Serial] Port opened at %d baud (8N1)\n", SERIAL_BAUD_RATE);
    return true;
}

extern "C" void platform_init(int argc, char **argv)
{
    (void)argc;
    (void)argv;
    bmda_probe_s probe_info = {};
    if (!find_debuggers(&probe_info))
    {
        fprintf(stderr, "[Serial] No compatible probe found.\n");
        exit(1);
    }
    bmp_ident(&probe_info);
    if (!serial_open())
    {
        fprintf(stderr, "[Serial] Failed to open probe serial port.\n");
        exit(1);
    }
}
