/**
 * @file dm_decoder.cpp
 * @brief This decodes DM requests & reply
 *
 */
#include "stdarg.h"
#include "stdint.h"
#include "stdio.h"
#include "string.h"

#include "bmp_logger.h"
#include <time.h>
#include <unistd.h>

#define KNRM "\x1B[0m"
#define KRED "\x1B[31m"
#define KGRN "\x1B[32m"
#define KBLU "\x1B[34m"
#define KMAG "\x1B[35m"
#define KCYN "\x1B[36m"
#define KWHT "\x1B[37m"

void decoderDMReg(int reg, uint32_t value);
#if 1
#undef QBMPLOG
#undef QBMPLOGN
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

/**
 * @brief
 *
 */
uint32_t _hex(uint8_t a)
{
    if (a >= '0' && a <= '9')
        return a - '0';
    if (a >= 'a' && a <= 'f')
        return a - 'a' + 10;
    if (a >= 'A' && a <= 'F')
        return a - 'A' + 10;
    return 0;
}
/**
 * @brief
 *
 * @param data
 * @param nbBytes
 * @return uint32_t
 */
uint32_t readLE(const uint8_t *data, const int nbBytes)
{
    uint32_t val = 0, shift = 0;
    ;
    for (int i = 0; i < nbBytes; i++)
    {
        int token = (_hex(data[0]) << 4) + _hex(data[1]);
        data += 2;
        val += token << shift;
        shift += 8;
    }
    return val;
}

typedef struct
{
    uint32_t num;
    const char *name;
} DM_registers;

#define DM_DATA0 0x04
#define DM_DATA11 0x0F
#define DM_STATUS 0x11
#define DM_CONTROL 0x10
#define DM_ABSTRACTCS 0x16
#define DM_ABSTRACTCMD 0x17

DM_registers dm_regs[] = {
    {0x04, "Abstract Data 0 (data0)"},
    {0x0f, "Abstract Data 11 (data11)"},
    {0x10, "Debug Module Control (dmcontrol)"},
    {0x11, "Debug Module Status (dmstatus)"},
    {0x12, "Hart Info (hartinfo)"},
    {0x13, "Halt Summary 1 (haltsum1)"},
    {0x14, "Hart Array Window Select (hawindowsel)"},
    {0x15, "Hart Array Window (hawindow)"},
    {0x16, "Abstract Control and Status (abstractcs)"},
    {0x17, "Abstract Command (command)"},
    {0x18, "Abstract Command Autoexec (abstractauto)"},
    {0x19, "Configuration String Pointer 0 (confstrptr0)"},
    {0x1a, "Configuration String Pointer 1 (confstrptr1)"},
    {0x1b, "Configuration String Pointer 2 (confstrptr2)"},
    {0x1c, "Configuration String Pointer 3 (confstrptr3)"},
    {0x1d, "Next Debug Module (nextdm)"},
    {0x20, "Program Buffer 0 (progbuf0)"},
    {0x2f, "Program Buffer 15 (progbuf15)"},
    {0x30, "Authentication Data (authdata)"},
    {0x34, "Halt Summary 2 (haltsum2)"},
    {0x35, "Halt Summary 3 (haltsum3)"},
    {0x37, "System Bus Address 127:96 (sbaddress3)"},
    {0x38, "System Bus Access Control and Status (sbcs)"},
    {0x39, "System Bus Address 31:0 (sbaddress0)"},
    {0x3a, "System Bus Address 63:32 (sbaddress1)"},
    {0x3b, "System Bus Address 95:64 (sbaddress2)"},
    {0x3c, "System Bus Data 31:0 (sbdata0)"},
    {0x3d, "System Bus Data 63:32 (sbdata1)"},
    {0x3e, "System Bus Data 95:64 (sbdata2)"},
    {0x3f, "System Bus Data 127:96 (sbdata3)"},
    {0x40, "Halt Summary 0 (haltsum0)"},
};

const char *lookup_dm_reg(int reg)
{
#define NB_DM_REG (sizeof(dm_regs) / sizeof(dm_regs[0]))
    for (int i = 0; i < NB_DM_REG; i++)
    {
        int v = dm_regs[i].num;
        if (v == reg)
            return dm_regs[i].name;
        if (v > reg)
            return "????";
    }
    return "????";
}
static int last_cmd = 0;
/**
 * @brief
 *
 * @param reg
 * @param value
 */
void decoderReply(int size, const uint8_t *data)
{
    if (!last_cmd)
    {
        QBMPLOG("?? not a reply ??");
        QBMPLOG("\n");
        last_cmd = 0;
        return;
    }
    if (data[0] == 'K')
    {
        QBMPLOG("  %s>OK%s\n", KGRN, KNRM);
    }
    else
    {
        QBMPLOG("  %s>FAIL!%s\n", KRED, KNRM);
    }
}

const char *regname(int reg)
{
    if (reg < 0x1000)
    {
        return "CSR";
    }
    if (reg < 0x1020)
    {
        return "GPR";
    }
    if (reg < 0x1040)
    {
        return "FPU";
    }
    return "?-?";
}
/**
 */
void decodeReply(int cmd, uint32_t value)
{
}
/**
 * @brief
 *
 * @param reg
 * @param value
 */
void decoderDMReg(int reg, uint32_t value)
{
    switch (reg)
    {
    case DM_STATUS:
        if (value & (1 << 5))
            QBMPLOG("resetReq");
        if (value & (1 << 8))
            QBMPLOG("anyhalted");
        if (value & (1 << 11))
            QBMPLOG("allrunning");
        break;
    case DM_DATA0:
        QBMPLOG("data0=0x%x", value);
        break;
    case DM_ABSTRACTCS:
        QBMPLOG("raw=0x%x progbufSize=%d words Err=%d datacount=%d ", value, (value >> 24) & 0xf, (value >> 8) & 0x3,
                (value & 0xf));
        break;
    case DM_ABSTRACTCMD: {
        int cmd = (value >> 24) & 0xff;
        int control = (value & 0xffffff);
        QBMPLOG("cmd=0x%x control=0x%x ", cmd, control);
        switch (cmd)
        {
        case 0:
            QBMPLOG(" RegisterAccess reg=%s (%d) write=%d autoinc=%d", regname(control & 0xffff), control & 0xffff,
                    (control >> 16) & 1, (control >> 19) & 1);
            break;
        case 1:
            QBMPLOG(" QuickAccess ");
            break;
        case 2: {
            bool write = (control >> 16) & 1, post_inc = (cmd >> 19) & 1;
            if (write)
            {
                QBMPLOG(" MemoryAccess arg0 -> [arg1] size=%d bytes postincrement=%d", (control >> 20) & 3, post_inc);
            }
            else
            {
                QBMPLOG(" MemoryAccess  [arg1] ->arg0 size=%d bytes write=%d postincrement=%d", (control >> 20) & 3,
                        post_inc);
            }
        }
        break;
        default:
            QBMPLOG("?-?");
            break;
        }
    }
    break;
    default:
        QBMPLOG("?-?");
        break;
    }
}
/**
 *
 *
 */
void decodeHL(int size, const uint8_t *data)
{
    last_cmd = 'H';
    switch (data[0])
    {
    case 'C':
        QBMPLOG("HL-Check Version\n");
        break;
    case 'A':
        QBMPLOG("HL-Accel \n");
        break;
    default:
        QBMPLOG("HL-?-? <%c< \n", data[0]);
        break;
    }
}

/**
 */
void decodeSWD(int size, const uint8_t *data)
{
    switch (data[0])
    {
    case 'S':
        QBMPLOG("SWD-RESET \n");
        break;
    default:
        QBMPLOG("SWD-?- ? \n");
        break;
    }
}
/**
 */
void decodeRV(int size, const uint8_t *data)
{
    last_cmd = 'G';
    switch (data[0])
    {
    case 'S':
        QBMPLOG("RV-RESET \n");
        break;
    case 'r':
        QBMPLOG("RV-DM READ \n");
        break;
    case 'w':
        QBMPLOG("RV-DM WRITE \n");
        break;
    default:
        QBMPLOG("RV-?- ? \n");
        break;
    }
}
/**
 */
void decodeGeneric(int size, const uint8_t *data)
{
    last_cmd = 'G';
    switch (data[0])
    {
    case 'f':
        QBMPLOG("Gen-FREQ GET \n");
        break;
    case 'F':
        QBMPLOG("Gen-FREQ SET \n");
        break;
    case 'Z':
        QBMPLOG("Gen-NRST SET \n");
        break;
    case 'z':
        QBMPLOG("Gen-NRST Clear \n");
        break;
    case 'A':
        QBMPLOG("Gen-Start RPC \n");
        break;
    case 'P':
        QBMPLOG("Gen-Power SET \n");
        break;
    case 'p':
        QBMPLOG("Gen-Power GET \n");
        break;
    case 'V':
        QBMPLOG("Gen-Voltage \n");
        break;
    case 'E':
        QBMPLOG("Gen-OutputEnable \n");
        break;
    default:
        QBMPLOG("Gen-?-? <%c< \n", data[0]);
        break;
    }
}
int _c(const uint8_t a)
{
    if (a >= 'A' && a <= 'F')
        return 10 + a - 'A';
    if (a >= 'a' && a <= 'f')
        return 10 + a - 'a';
    if (a >= '0' && a <= '9')
        return a - '0';
    return 0;
}
int _hex(const uint8_t *a)
{
    return _c((a[0]) << 4) + _c(a[1]);
}
/**
 */
void decodeMon(int size, const uint8_t *data)
{
    size >>= 1;
    for (int i = 0; size; i++)
    {
        QBMPLOG("%c", _hex(data));
        data += 2;
    }
    QBMPLOG("\n");
}
/**
 */
void decodeLNADIV5(int size, const uint8_t *data)
{
    switch (data[0])
    {
    case 'W':
        QBMPLOG("LnAdiv5- Write\n");
        break;
    case 'R':
        QBMPLOG("LnAdiv5- Read\n");
        break;
    case 'A':
        QBMPLOG("LnAdiv5- RAW\n");
        break;
    case 'B':
        QBMPLOG("LnAdiv5- LowLevel\n");
        break;
    default:
        QBMPLOG("LnAdiv5-???? \n");
        break;
    }
}
void decodeADIV5(int size, const uint8_t *data)
{
    switch (data[0])
    {
    case 'd':
        QBMPLOG("Adiv5-DP READ \n");
        break;
    case 'D':
        QBMPLOG("Adiv5-DP WRITE \n");
        break;
    case 'a':
        QBMPLOG("Adiv5-AP READ \n");
        break;
    case 'A':
        QBMPLOG("Adiv5-AP WRITE \n");
        break;
    case 'R':
        QBMPLOG("Adiv5-RAW ACCESS \n");
        break;

    default:
        QBMPLOG("Adiv5-???? \n");
        break;
    }
}

/**
 *
 *
 */
static bool first_frame = true;
void decoderRequest(int size, const uint8_t *data)
{
    if (size < 3)
        return;
    switch (data[0])
    {
    case '!':
        break;
    case '$':
        decodeMon(size - 2, data + 2);
        break;
    default:
        if (!first_frame)
        {
            QBMPLOG("?-?-invalid frame <%s>\n", data);
            return;
        }
        first_frame = false;
        last_cmd = 1;
        break;
    }
    QBMPLOG("RPC CALL:");
    switch (data[1])
    {
    case 'O':
        decodeMon(size - 2, data);
        return;
        break;
    case 'J':
        QBMPLOG("Jtag-unsupported");
        return;
        break;
    case 'S':
        decodeSWD(size - 2, data + 2);
        return;
        break;
    case 'B':
        decodeRV(size - 2, data + 2);
        return;
        break;
    case 'G':
        decodeGeneric(size - 2, data + 2);
        return;
        break;
    case 'H':
        decodeHL(size - 2, data + 2);
        return;
        break;
    case 'A':
        decodeADIV5(size - 2, data + 2);
        return;
        break;
    case 'L':
        decodeLNADIV5(size - 2, data + 2);
        return;
        break;
    case 'Z':
        QBMPLOG("SWINDLE-unsupported");
        return;
        break;
    case 's':
        QBMPLOG("SPI-unsupported");
        return;
        break;
    default:
        QBMPLOG("?-?-unsupported");
        return;
        break;
    }
#if 0
        case 'm':
            QBMPLOG("Mem read ");
            break;
        case 'M':
            QBMPLOG("Mem write ");
            break;
        case 'B': {
            switch (data[2])
            {
            case 'r': {
                QBMPLOG("DMI READ ");
                uint32_t reg = readLE(data + 3, 1);
                QBMPLOG("Reg %s (0x%x) ", lookup_dm_reg(reg), reg);
                last_cmd = reg;
            }
            break;

            case 'w': {
                QBMPLOG("DMI WRITE ");
                // What's after is the DMI : Register 8 bits is enough but we use 32 then 32 bits data
                uint32_t reg = readLE(data + 3, 4);
                uint32_t value = readLE(data + 3 + 4 * 2, 4);
                QBMPLOG("Reg %s (0x%x) Data=0x%x ", lookup_dm_reg(reg), reg, value);
                decoderDMReg(reg, value);
                last_cmd = 0;
            }
            break;
            default:
                QBMPLOG("DMI ?? ");
                break;
            }
        }
        break;
        default:
            QBMPLOG(" ???");
            break;
        }
    }
#endif
}
#if 0
extern "C" int bmda_usb_transfer(usb_link_s *link, const void *tx_buffer, size_t tx_len, void *rx_buffer, size_t rx_len,
                                 uint16_t timeout)
{
    return -1;
}
#endif
// EOF
