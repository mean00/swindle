#pragma once
#if 1
#define GDB_LOGGER(...)                                                                                                \
    {                                                                                                                  \
    }
#else
#define GDB_LOGGER Logger
#endif

class Gdb
{
  public:
    static bool startGatheringSymbol();
    // Ask the current thread
    static void Qc();
    //
    static void threadInfo(uint32_t threadId);
    //
    static bool decodeSymbol(int len, const char *packet);
    static bool switchThread(uint32_t threadId);
    static bool isThreadAlive(uint32_t threadId);
    static uint32_t getCurrentThreadId();
};
// EOF
