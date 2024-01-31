/*
https://ftp.gnu.org/old-gnu/Manuals/gdb/html_chapter/gdb_15.html
http://web.mit.edu/rhel-doc/3/rhel-gdb-en-3/general-query-packets.html

qTStatus
qAttached
qP0000001f0000000000007fff
qL120000000000

OFFSET_LIST_ITEM_NEXT = 4,
OFFSET_LIST_ITEM_OWNER = 12,
OFFSET_LIST_NUMBER_OF_ITEM = 0,
OFFSET_LIST_INDEX = 4,
NB_OF_PRIORITIES = 16,
MPU_ENABLED = 0,
MAX_TASK_NAME_LEN = 16,
OFFSET_TASK_NAME = 52,
OFFSET_TASK_NUM = 68}

   The following is assumed
   - Everything starts with a qfthread info or qC
   - The number of threads is not too high
   - We cache the TCB and thread # between 2 calls


 */

#include "bmp_string.h"
#include "lnArduino.h"
extern "C"
{
#include "gdb_packet.h"
#include "general.h"
#include "hex_utils.h"
#include "lnFreeRTOSDebug.h"
#include "target.h"
#include "target_internal.h"
}
#include "bmp_util.h"

AllSymbols allSymbols;
lnThreadInfoCache *threadCache = NULL;

#define TARGET_READY()                                                                                                 \
    {                                                                                                                  \
        if (!(allSymbols.ready() && cur_target))                                                                       \
        {                                                                                                              \
            gdb_putpacketz("");                                                                                        \
            return true;                                                                                               \
        }                                                                                                              \
    }

/**

*/
void initFreeRTOS()
{
    if (!threadCache)
    {
        threadCache = new lnThreadInfoCache;
        allSymbols.clear();
    }
}

#include "bmp_gdb_cmd.h"

#define STUBFUNCTION_END(x)                                                                                            \
    bool x(const char *packet, int len)                                                                                \
    {                                                                                                                  \
        Logger("::: %s:%s\n", #x, packet);                                                                             \
        gdb_putpacket("l", 1);                                                                                         \
        return true;                                                                                                   \
    }
#define STUBFUNCTION_EMPTY(x)                                                                                          \
    bool x(const char *packet, int len)                                                                                \
    {                                                                                                                  \
        Logger("::: %s:%s\n", #x, packet);                                                                             \
        gdb_putpacketz("");                                                                                            \
        return true;                                                                                                   \
    }

/**
 *  qC  : Get current thread
 */
bool exect_qC(const char *packet, int len)
{
    Logger("::: exect_qC:%s\n", packet);
    initFreeRTOS(); // host mode
    TARGET_READY();
    threadCache->updateCache();
    Gdb::Qc();
    return true;
}

/*
    qSymbol Grab FreeRTOS symbols
*/
bool execqSymbol(const char *packet, int len)
{
    Logger(":::<execqSymbol>:<%s>\n", packet);
    if (len == 1 &&
        packet[0] == ':') // :: : ready to serve https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
    {
        Gdb::startGatheringSymbol();
        return true;
    }
    if (len > 1) // actual data
    {
        Gdb::decodeSymbol(len, packet);
        return true;
    }
    gdb_putpacket("OK", 2);
    return true;
}
/**
 *  qOffset: grab symbol offset (there is none)
 */

bool execqOffsets(const char *packet, int len)
{
    Logger("::: execqOffsets:%s\n", packet);
    // it's xip, no offset...
    gdb_putpacket("Text=0;Data=0;Bss=0", 19); // 7 7 5=>19
    return true;
}

STUBFUNCTION_END(execqsThreadInfo)
STUBFUNCTION_EMPTY(execqThreadInfo)

/**
 *  execqfThreadInfo get a list of threads
 * */

bool execqfThreadInfo(const char *packet, int len)
{
    Logger("::: qfThreadinfo:%s\n", packet);
    TARGET_READY();

    threadCache->updateCache();
    stringWrapper wrapper;
    threadCache->collectIdAsWrapperString(wrapper);
    char *out = wrapper.string();
    if (strlen(out))
    {
        gdb_putpacket2("m", 1, out, strlen(out));
        Logger(out);
    }
    else
    {
        gdb_putpacket("m0", 2);
    }
    free(out);
    return true;
}
/**
 *  \fn exect_qThreadExtraInfo
 *  \brief Get extra info as a hex string
 */
bool exect_qThreadExtraInfo(const char *packet, int len)
{
    Logger("::: exect_qThreadExtraInfo:%s\n", packet);
    TARGET_READY();
    uint32_t tid;
    if (1 != sscanf(packet, ",%x", &tid))
    {
        Logger("Invalid thread info\n");
        gdb_putpacketz("E01");
        return true;
    }
    Gdb::threadInfo(tid);
    return true;
}
/**
 *  \fn exec_H_cmd
 *  \brief switch thread
 * */
bool exec_H_cmd(const char *packet, int len)
{
    Logger("::: exec_H_cmd:<%s>\n", packet);
    TARGET_READY();
    int tid;
    if (1 != sscanf(packet, "%d", &tid))
    {
        Logger("Invalid thread id\n");
        gdb_putpacketz("E01");
        return false;
    }
    if (0 == tid)
    {
        Logger("Invalid thread id\n");
        gdb_putpacketz("OK");
        return false;
    }
    if (!Gdb::switchThread(tid))
    {
        gdb_putpacketz("E01");
        return false;
    }
    return true;
}

/**
 * \fn exec_H_cmd2
 * \brief stub for some Hxx commands, does nothing
 * */
bool exec_H_cmd2(const char *packet, int len)
{
    Logger("::: exec_H_cmd2:<%s>\n", packet);
    TARGET_READY();
    gdb_putpacketz("OK");
    return true;
}

/**
 *  \fn exec_T_cmd
 *  \brief  Ask if the thread is alive
 * */
bool exec_T_cmd(const char *packet, int len)
{
    Logger("::: exec_T_cmd:<%s>\n", packet);
    TARGET_READY();
    int tid;
    if (1 != sscanf(packet, "%d", &tid))
    {
        Logger("Invalid thread id\n");
        gdb_putpacketz("E01");
        return true;
    }
    if (0 == tid)
    {
        Logger("Invalid thread id\n");
        gdb_putpacketz("E01");
        return true;
    }
    fdebug2("Thread : %d\n", tid);
    if (!Gdb::isThreadAlive(tid))
    {
        gdb_putpacketz("E01");
        return true;
    }
    else
    {
        gdb_putpacketz("OK");
        return true;
    }
}

//
//  Callback structures
//
typedef struct
{
    const char *cmd_prefix;
    bool (*func)(const char *packet, int len);
    const bool exactMatch;
} ln_cmd_executer;

typedef struct
{
    const char c;
    const ln_cmd_executer *cmds;
} PrefixedCommands;

extern bool exec_H_cmd(const char *packet, int len);
extern bool exec_T_cmd(const char *packet, int len);
/**
 *
 * */
static const ln_cmd_executer H_commands[] = {
    {"Hg", exec_H_cmd, false},
    {"Hm", exec_H_cmd, false},
    {"HM", exec_H_cmd, false},
    {"Hc", exec_H_cmd, false},
    {NULL, NULL},
};

static const ln_cmd_executer T_commands[] = {
    {"T", exec_T_cmd, false},
    {NULL, NULL},
};

static const ln_cmd_executer q_commands[] = {
    {"qOffsets", execqOffsets, false},
    {"qSymbol:", execqSymbol, false},
    {"qThreadInfo", execqThreadInfo, false},
    {"qfThreadInfo", execqfThreadInfo, false},
    {"qsThreadInfo", execqsThreadInfo, false},
    {"qThreadExtraInfo", exect_qThreadExtraInfo, false},
    {"qC", exect_qC, true}, // needs to be a an exact match else it is confused with qCRC
    {NULL, NULL},
};

/**
 *  \brief per letter table
 * */
PrefixedCommands prefixedCommands[] = {{'q', q_commands}, {'H', H_commands}, {'T', T_commands}, {0, NULL}};

enum lnExecResult
{
    lnExecUnsupported = 0,
    lnExecOk = 1,
    lnExecError = 2,
};

/**
 *
 * */
lnExecResult ln_exec_command(const char *packet, int len, const ln_cmd_executer *exec)
{
    while (exec->cmd_prefix)
    {
        int l = strlen(exec->cmd_prefix);
        if (!strncmp(packet, exec->cmd_prefix, l))
        {
            bool toRun = true;
            if (exec->exactMatch)
            {
                if (len != strlen(exec->cmd_prefix))
                {
                    toRun = false;
                }
            }
            if (toRun)
            {
                if (exec->func(packet + l, len - l))
                {
                    return lnExecOk;
                }
                else
                {
                    Logger("*** malformed command %s\n", packet);
                    return lnExecError;
                }
            }
            else
            {
                return lnExecUnsupported;
            }
        }
        exec++;
    }
    return lnExecUnsupported;
}
/**
 *
 **/
extern "C" bool lnInterceptCommand(const char *packet)
{

    uint8_t c = packet[0];
    PrefixedCommands *cmd = prefixedCommands;
    Logger(">> cmd [%c][%c]\n", packet[0], packet[1]);
    while (cmd->c)
    {
        if (cmd->c == c)
        {
            lnExecResult er = ln_exec_command(packet, strlen(packet), cmd->cmds);
            switch (er)
            {
            case lnExecOk:
            case lnExecError:
                return true;
                break;
            case lnExecUnsupported:
            default:
                return false;
                break;
            }
        }
        cmd++;
    }
    return false;
}

//
