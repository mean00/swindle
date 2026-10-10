/**
 * @file bmp_qtserial.cpp
 * @brief Qt serial port transport implementation for swindle_hosted.
 */

#include <cstdarg>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <unistd.h>

extern "C"
{
#include "../hosted/bmp_hosted.h"
#include "../hosted/cli.h"
}

#include <QDateTime>
#include <QSerialPort>
#include <QSerialPortInfo>

#include "bmp_logger.h"
#include "hosted_constants.h"

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
static bool readChar(uint8_t *data)
{
    if (s_serial_head == s_serial_tail)
    {
        s_serial_head = s_serial_tail = 0;
    }

    while (true)
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

        int available = s_serial_port->bytesAvailable();
        if (available > 0)
        {
            if (available > SERIAL_BUFFER_SIZE)
            {
                available = SERIAL_BUFFER_SIZE;
            }
            int bytes_read = s_serial_port->read(reinterpret_cast<char *>(s_serial_ring_buffer + s_serial_tail), available);
            if (bytes_read > 0)
            {
                s_serial_tail += bytes_read;
                continue;
            }
            if (bytes_read < 0)
            {
                fprintf(stderr, "[Serial] Read error encountered on port\n");
                return false;
            }
        }

        if (!s_serial_port->waitForReadyRead(SERIAL_WAIT_TIMEOUT_MS))
        {
            // Timeout: continue waiting
        }
    }
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

    while (true)
    {
        uint8_t c = 0;
        if (!readChar(&c))
        {
            return SERIAL_READ_ERROR_IO;
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
    return 0;
}

/**
 * @brief Find and enumerate compatible BMP/Swindle probes connected via USB serial.
 */
extern "C" bool find_debuggers(bmda_cli_options_s *cl_opts, bmda_probe_s *info)
{
    (void)cl_opts;
    const auto ports = QSerialPortInfo::availablePorts();

    for (const QSerialPortInfo &portInfo : ports)
    {
        if (!portInfo.hasVendorIdentifier() || !portInfo.hasProductIdentifier())
        {
            continue;
        }

        quint16 vid = portInfo.vendorIdentifier();
        quint16 pid = portInfo.productIdentifier();

        if (vid == USB_VID_OPENMOKO)
        {
            bool is_supported_probe = (pid == USB_PID_SWINDLE_LN ||
                                       pid == USB_PID_SWINDLE_PICO ||
                                       pid == USB_PID_BMP_CLASSIC);
            if (is_supported_probe)
            {
                memset(info, 0, sizeof(*info));
                info->type = PROBE_TYPE_BMP;
                strncpy(info->manufacturer, portInfo.systemLocation().toLatin1().constData(), sizeof(info->manufacturer) - 1);

                printf("Found Probe: VID=0x%04x, PID=0x%04x on %s\n", vid, pid, info->manufacturer);
                switch (pid)
                {
                case USB_PID_SWINDLE_LN:
                    printf("\tModel: Swindle GD32/CH32 Debugger\n");
                    break;
                case USB_PID_SWINDLE_PICO:
                    printf("\tModel: pSwindle RP2040 Pico Debugger\n");
                    break;
                case USB_PID_BMP_CLASSIC:
                    printf("\tModel: Classic Black Magic Probe\n");
                    break;
                default:
                    printf("\tModel: OpenMoko / BMP Debugger\n");
                    break;
                }
                return true;
            }
        }
    }

    fprintf(stderr, "[Serial] No compatible Black Magic / Swindle probe found.\n");
    return false;
}

/**
 * @brief Clean up and close the serial port on shutdown.
 */
extern "C" void libusb_exit_function(bmda_probe_s *info)
{
    (void)info;
    if (s_serial_port)
    {
        s_serial_port->close();
        delete s_serial_port;
        s_serial_port = nullptr;
    }
}

/**
 * @brief Create the serial port instance for the probe.
 */
extern "C" void bmp_ident(bmda_probe_s *info)
{
    printf("[Serial] Selecting port: %s\n", info->manufacturer);
    s_serial_port = new QSerialPort(QString(info->manufacturer));
}

/**
 * @brief Open and configure the serial port for 8N1 communication.
 */
extern "C" bool serial_open(const bmda_cli_options_s *opt, const char *serial)
{
    (void)opt;
    (void)serial;

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
    s_serial_port->setReadBufferSize(1);

    printf("[Serial] Port opened at %d baud (8N1)\n", SERIAL_BAUD_RATE);
    return true;
}