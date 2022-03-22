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


 */
 #include "lnArduino.h"
 #include "bmp_string.h"
 extern "C"
 {
 #include "hex_utils.h"
 #include "target.h"
 #include "target_internal.h"
 #include "gdb_packet.h"
 #include "lnFreeRTOSDebug.h"
}
extern target *cur_target;
extern "C" void gdb_putpacket(const char *packet, int size);

#define fdebug2(...) {}
#define fdebug Logger
/**

*/
#define O(x) allSymbols._debugInfo.x
uint32_t readMem32(uint32_t base, uint32_t offset)
{
  return target_mem_read32(cur_target,base+offset);
}

#include "bmp_symbols.h"
AllSymbols allSymbols;
#include "bmp_freertos_tcb.h"
/**

*/
void initFreeRTOS()
{
  allSymbols.clear();
}

// The TCB structure has the following layout
// 0 *pxTopOfStack <= current stack
//      (MPU) we ignore that for now
// 4 List xStateListItem
// 8 List xEventListItem
// uxPriority
// pxStack <=original stack
// name[]
// ...
// uxTCBNumber <= uniq task number


//  List of ready tsk prio 0
//....
//  List of ready tsk prio 15

class listThread : public ThreadParserBase
{
public:
    listThread(stringWrapper *w)
    {
      _w=w;
    }
    void execList(FreeRTOSSymbols state,uint32_t tcbAdr)
    {
        uint32_t id=readMem32(tcbAdr,O(OFFSET_TASK_NUM));
        if(strlen(_w->string()))
          _w->append(",");
        _w->appendHex64(id);
    }
protected:
    stringWrapper *_w;
};

/**

*/

class findThread : public ThreadParserBase
{
public:
    findThread(uint32_t  threadId)
    {
      _threadId=threadId;
      _tcbAddress=0;
    }
    void execList(FreeRTOSSymbols state,uint32_t tcbAdr)
    {
        uint32_t id=readMem32(tcbAdr,O(OFFSET_TASK_NUM));
        if(id==_threadId)
        {
          _tcbAddress=tcbAdr;
          _symbol=state;
        }
    }
    uint32_t        tcb()     {return _tcbAddress;}
    FreeRTOSSymbols symbol()  {return _symbol;};
protected:
  uint32_t  _threadId;
  uint32_t  _tcbAddress;
  FreeRTOSSymbols _symbol;
};

/**

*/
class Gdb
{
public:
  //

  static bool startGatheringSymbol()
  {
    allSymbols.clear();
    allSymbols.queryNextSymbol();
    return true;
  }

  // Ask the current thread
  static void Qc()
  {
    uint32_t pxCurrentTcb;
    if(!allSymbols.readSymbolValue(spxCurrentTCB,pxCurrentTcb))
    {
      gdb_putpacketz("");
      return;
    }
    uint32_t tid_adr=pxCurrentTcb;
    Logger("Current TID ADR=%x\n",tid_adr);
    uint32_t threadId=readMem32(tid_adr,O(OFFSET_TASK_NUM)); //68
    Logger("Current TID =%x\n",threadId);

    char tst[10+3];

    if(!threadId) threadId=2;
    sprintf(tst,"QC%x",threadId);
    gdb_putpacket(tst,strlen(tst));
    Logger(tst);

  }
  //
  static void threadInfo(uint32_t  threadId)
  {
    findThread fnd(threadId);
    fnd.run();
    uint32_t tcb=fnd.tcb();
    FreeRTOSSymbols sym=fnd.symbol();
    if(!tcb) // assuming zero is not a valid address
    {
          gdb_putpacketz("E01");
          return;
    }
    //
    stringWrapper wrapper;

    int maxLen=O(MAX_TASK_NAME_LEN);
    uint32_t name=tcb+52;
    char taskName[maxLen+1];
    target_mem_read(cur_target,taskName,name,maxLen);
    taskName[maxLen]=0;
    wrapper.append(taskName);

    const char *st;
    switch(sym) // The queue it has been pulled from, gives the task state
    {
      case spxCurrentTCB:           st="R";break;
      case sxSuspendedTaskList:     st="S";break;
      case spxDelayedTaskList:      st="D";break;
      case spxReadyTasksLists:      st="r";break;
      default: st="?";break;
    }
    // Get task name now
    wrapper.append("[");
    wrapper.append(st);
    wrapper.append("]");

    wrapper.append("  TCB: 0x");
    wrapper.appendHex32(tcb);


    char *in=wrapper.string();;
    int l=strlen(in);
    char out[2*l+1];
    hexify(out,in,l);
    Logger(out);
    gdb_putpacket(out, 2*l);
  }
  //
  static bool decodeSymbol(int len, const char *packet)
  {
    allSymbols.decodeSymbol(len, packet);
    return true;
  }

};


#define PRE_CHECK_DEBUG_TARGET(sym)  { if(!allSymbols.readDebugBlock ()) \
                                      {    gdb_putpacketz("");  \
                                          return;  } }

#define STUBFUNCTION_END(x)  extern "C" void x(const char *packet, int len) \
{ \
  Logger("::: %s:%s\n",x,packet); \
  gdb_putpacket("l", 1); \
}
#define STUBFUNCTION_EMPTY(x)  extern "C" void x(const char *packet, int len) \
{ \
  Logger("::: %s:%s\n",x,packet); \
  gdb_putpacketz(""); \
}
/**

*/
extern "C" void exect_qC(const char *packet, int len)
{
  Logger("::: exect_qC:%s\n",packet);
  PRE_CHECK_DEBUG_TARGET();
  Gdb::Qc();
}

/*
    Grab FreeRTOS symbols
*/
extern "C" void execqSymbol(const char *packet, int len)
{
  Logger("<execqSymbol>:<%s>\n",packet);
  if(len==1 && packet[0]==':') // :: : ready to serve https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
  {
    Gdb::startGatheringSymbol();
    return;
  }
  if(len>1)
  {
      Gdb::decodeSymbol(len,packet);
      return;
  }
  gdb_putpacket("OK", 2);
}
/**

*/
extern "C" bool lnProcessCommand(int size, const char *data)
{
    Logger("Received packet %s\n",data);
    return false;
}
//
extern "C" void execqOffsets(const char *packet, int len)
{
  // it's xip...
  gdb_putpacket("Text=0;Data=0;Bss=0", 19); // 7 7 5=>19
}


STUBFUNCTION_END(execqsThreadInfo)

STUBFUNCTION_EMPTY(execqThreadInfo)
/**

*/
extern "C" void execqfThreadInfo(const char *packet, int len)
{
  Logger("::: qfThreadinfo:%s\n",packet);
  PRE_CHECK_DEBUG_TARGET();

  stringWrapper wrapper;
  listThread list(&wrapper); // list all the threads
  list.run();
  char *out=wrapper.string();
  if(strlen(out))
  {
    gdb_putpacket2("m",1,out,strlen(out));
    Logger(out);
    free(out);
  }else
  {
    // Grab all the threads in one big array
    Logger("m 0");
    gdb_putpacket("m0", 2);
  }
}

/**

*/
extern "C" void exect_qThreadExtraInfo(const char *packet, int len)
{
  Logger("::: exect_qThreadExtraInfo:%s\n",packet);
  PRE_CHECK_DEBUG_TARGET();
  uint32_t tid;
  if(1!=sscanf(packet,",%x",&tid))
  {
     Logger("Invalid thread info\n");
	   gdb_putpacketz("E01");
     return;
  }
  Gdb::threadInfo(tid);
}
//
