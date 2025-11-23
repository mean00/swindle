#include "stdarg.h"
#include "stdint.h"
#include "stdio.h"
#include "string.h"

#include <time.h>
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

#define BMP_VID 0x1d50
#define BMP_PID 0x6018
// RP2040
#define LNBMP_PID 0x6050
// CH/GD
// #define LNBMP_PID 0x6030

#define WCHLINK_VID 0x1a86
#define WCHLINK_PID 0x8010

#define TAG 0x44

#define xAssert(x)                                                                                                     \
    if (!(x))                                                                                                          \
    {                                                                                                                  \
        printf("%s FAILED in %s \n", #x, __PRETTY_FUNCTION__);                                                         \
        exit(0);                                                                                                       \
    }

//-- disable debug
#if 1
#undef QBMPLOG
#undef QBMPLOGN
#undef QBMPLOGH
#define QBMPLOG(...)                                                                                                   \
    {                                                                                                                  \
    }
#define QBMPLOGH(...)                                                                                                  \
    {                                                                                                                  \
    }
#define QBMPLOGN(...)                                                                                                  \
    {                                                                                                                  \
    }
#define QCALL(...)                                                                                                     \
    {                                                                                                                  \
    }
#else
#define QCALL(x) x
#endif

QSerialPort *qserial = NULL;
extern "C" void platform_write_flush();
extern "C" int platform_buffer_write_buffered(const uint8_t *data, int size);
extern "C" int platform_buffer_read(uint8_t *data, int maxsize);
extern "C" int platform_buffer_write(const uint8_t *data, int size);
void decoderRequest(int size, const uint8_t *data);
void decoderReply(int size, const uint8_t *data);

/**
 */
#define PLATFORM_BUFFER_SIZE 512
static int plf_write_index = 0;
static uint8_t platform_buffer[PLATFORM_BUFFER_SIZE];
/**
 */

bool waitData()
{
    while (1)
    {
        int a = qserial->bytesAvailable();
        if (a == 0)
        {
            while (qserial->waitForReadyRead(10) == false)
            {
            }
            continue;
        }
        if (a < 0)
        {
            return false;
        }
        return true;
    }
}

enum SER_AUTO
{
    SERIAL_IDLE,
    SERIAL_DATA,
    SERIAL_DONE
};
#define SERIAL_BUFFER_SIZE 512
uint8_t buffer_serial[SERIAL_BUFFER_SIZE];
SER_AUTO serial_auto = SERIAL_IDLE;
int serial_head = 0;
int serial_tail = 0;

bool readChar(uint8_t *data)
{
    // shrink
    if (serial_head == serial_tail)
    {
        serial_head = serial_tail = 0;
    }
    while (1)
    {
        if (serial_head != serial_tail)
        {
            *data = buffer_serial[serial_head++];
            return true;
        }

        // do we have data already available ?
        int a = qserial->bytesAvailable();
        if (a)
        {
            if (a > SERIAL_BUFFER_SIZE)
            {
                a = SERIAL_BUFFER_SIZE;
            }
            int nb = qserial->read((char *)(buffer_serial + serial_tail), a);
            if (nb > 0)
            {
                serial_tail += nb;
                continue;
            }
            if (nb < 0)
            {
                QBMPERROR("Error on serial\n");
                return false;
            }
        }
        while (qserial->waitForReadyRead(10) == false)
        {
        }
    }
}
/**
 *
 *
 */
extern "C" int platform_buffer_write_buffered(const uint8_t *data, int size)
{
    if (size + plf_write_index >= PLATFORM_BUFFER_SIZE)
        platform_write_flush();

    if (size + plf_write_index >= PLATFORM_BUFFER_SIZE)
        xAssert(0);
    memcpy(platform_buffer + plf_write_index, data, size);
    plf_write_index += size;
    return size;
}
/**
 *
 */
extern "C" void platform_write_flush()
{
    if (!plf_write_index)
        return;
    platform_buffer_write(platform_buffer, plf_write_index);
    plf_write_index = 0;
}

/**
 *
 */
extern "C" int platform_buffer_read(uint8_t *data, int maxsize)
{
    // This breaks layers separation oh well
    // The expected input is REMOTE_RESP ... REMOTE_EOM
    // and we take only the payload
    // too bad we have a proper automaton in the rust side
    serial_auto = SERIAL_IDLE;
    int nb = 0;
    while (1)
    {
        uint8_t c;
        if (!readChar(&c))
        {
            return -6;
        }
        uint8_t altc = c;
        if (altc < ' ')
            altc = ' ';
        //        QBMPERROR("**< rx %c <0x%x> \n",altc,c);
        switch (serial_auto)
        {
        case SERIAL_IDLE:
            if (c == '&')
            {
                serial_auto = SERIAL_DATA;
                break;
            }
            else
            {
                QBMPERROR("*************Warning : invalid resp <0x%x> \n", serial_auto);
            }
            break;
        case SERIAL_DATA: {

            if (c == '#')
            {
                data[nb] = 0;
                QBMPLOG("Serial Read %d bytes ", nb);
                QBMPLOGN(nb, (const char *)data);
                QBMPLOG("\n");
                serial_auto = SERIAL_DONE;
                //               QBMPERROR("*----*\n");
                QCALL(decoderReply(nb, data));
                return nb;
            }
            data[nb++] = c;
            if (nb > maxsize)
            {
                QBMPLOG("OVERFLOW!\n");
                return -6;
            }
        }
        break;
        case SERIAL_DONE:
        default:
            QBMPERROR("Invalid state\n");
            xAssert(0);
            break;
        }
    }
}
/**
 */

extern "C" int platform_buffer_write(const uint8_t *data, int size)
{
    int orgsize = size;
    QCALL(decoderRequest(orgsize, data));
    while (size)
    {
        int nb = qserial->write((const char *)data, size);
        if (nb <= 0)
        {
            QBMPERROR("***** WRITE FAILURE\n");
            exit(-1);
            break;
        }
        // QBMPLOG("Write %d bytes", nb);
        QBMPLOGH("Write:");
        QBMPLOGN(nb, (const char *)data);
        QBMPLOG("(%d bytes)\n", nb);
        size -= nb;
        data += nb;
    }
    // QBMPLOG("-- flush in \n");
    qserial->flush();
    // QBMPLOG("-- flush end \n");
    // QBMPLOG("Write %d bytes\n", orgsize);
    return size;
}
/**
 */

extern "C" bool find_debuggers(bmda_cli_options_s *cl_opts, bmda_probe_s *info)
{
#if 0
    const int result = libusb_init(&info->libusb_ctx);
    if (result != LIBUSB_SUCCESS)
    {
        printf("Failed to initialise libusb (%d): %s\n", result, libusb_error_name(result));
        return false;
    }
#endif
    const auto serialPortInfos = QSerialPortInfo::availablePorts();
    for (const QSerialPortInfo &portInfo : serialPortInfos)
    {
        if (portInfo.hasVendorIdentifier() && portInfo.hasProductIdentifier())
        {
            quint16 pid = portInfo.productIdentifier();
            switch (portInfo.vendorIdentifier())
            {
            case BMP_VID: {
                // take the 1st one (?)
                if ((pid == LNBMP_PID) || pid == BMP_PID || pid == LNBMP_PID)
                {
                    // take the 1st one (?)
                    memset(info, 0, sizeof(*info));
                    info->type = PROBE_TYPE_BMP;
                    // this is ugly, dont check anything just copy hoping it fits
                    strcpy(info->manufacturer, portInfo.systemLocation().toLatin1().constData());
                    printf("Found LNBMP\n");
                    return true;
                }
            }
            break;
#if 0
            case WCHLINK_VID:
                if (pid == WCHLINK_PID)
                {
                    // take the 1st one (?)
                    // memset(info, 0, sizeof(*info));
                    info->type = PROBE_TYPE_WCHLINK;
                    // this is ugly, dont check anything just copy hoping it fits
                    strcpy(info->manufacturer, "wchlink");
                    info->pid = WCHLINK_PID;
                    info->vid = WCHLINK_VID;
                    printf("Found WCHLINK\n");
                    const int result = libusb_init(&info->libusb_ctx);
                    if (result != LIBUSB_SUCCESS)
                    {
                        printf("Error initializing libusb\n");
                        exit(-1);
                    }
                    return 0;
                }
                break;
#endif
            }
        }
    }
    QBMPERROR("-- Cannot find valid debugger --\n");
    xAssert(0);
    return true;
}
/**
 */

extern "C" void libusb_exit_function(bmda_probe_s *info)
{
    QBMPERROR("Exiting libusb..\n");
    if (qserial)
    {
        qserial->close();
        delete qserial;
        qserial = NULL;
    }
}
/**
 */
extern "C" void bmp_ident(bmda_probe_s *info)
{
    QBMPLOG("Probing : %s\n", info->manufacturer);

    QString port(info->manufacturer);
    qserial = new QSerialPort(port);
}
/**
 */

extern "C" bool serial_open(const bmda_cli_options_s *opt, const char *serial)
{
    QBMPLOG("Serial open\n");
    if (!qserial->open(QIODevice::ReadWrite))
    {
        QBMPERROR("cannot open\n");
        return false;
    }
    qserial->setFlowControl(QSerialPort::NoFlowControl);
    qserial->setBaudRate(QSerialPort::Baud115200);
    qserial->setDataBits(QSerialPort::Data8);
    qserial->setReadBufferSize(1);
    // qserial->setFlowControl(QSerialPort::NoFlowControl);
    return true;
}
#if 0
extern "C" int bmda_usb_transfer(usb_link_s *link, const void *tx_buffer, size_t tx_len, void *rx_buffer, size_t rx_len,
                                 uint16_t timeout)
{
    return -1;
}
#endif
// EOF
