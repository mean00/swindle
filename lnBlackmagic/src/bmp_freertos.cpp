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
  if(!target_validate_address_flash_or_ram(cur_target,base))
  {
    Logger("Invalid ram read %x+%x\n",base,offset);
    return 0;
  }
  return target_mem_read32(cur_target,base+offset);
}
void writeMem32(uint32_t base, uint32_t offset,uint32_t value)
{
  if(!target_validate_address_flash_or_ram(cur_target,base))
  {
    Logger("Invalid ram read %x+%x\n",base,offset);
    return ;
  }
  target_mem_write32(cur_target,base+offset,value);
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
        //Logger("     TCB %x \n",tcbAdr);
        uint32_t id=readMem32(tcbAdr,O(OFFSET_TASK_NUM));
        //Logger("        id %d \n",id);
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

#include "bmp_cortex_registers.h"

#include "bmp_gdb.h"

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
  }else
  {
    // Grab all the threads in one big array
    Logger("m 0");
    gdb_putpacket("m0", 2);
  }
  free(out);
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

extern "C" void exec_H_cmd(const char *packet, int len)
{
    Logger("::: exec_H_cmd:<%s>\n",packet);
    PRE_CHECK_DEBUG_TARGET();
    int tid;
    if(1!=sscanf(packet,"%d",&tid))
    {
       Logger("Invalid thread id\n");
       gdb_putpacketz("E01");
       return;
    }
    if(0==tid)
    {
      Logger("Invalid thread id\n");
      gdb_putpacketz("E01");
      return;
    }    
    if(!Gdb::switchThread(tid))
    {
      gdb_putpacketz("E01");
      return;
    }
}

extern "C" void exec_T_cmd(const char *packet, int len)
{
    Logger("::: exec_T_cmd:<%s>\n",packet);
    PRE_CHECK_DEBUG_TARGET();
    int tid;
    if(1!=sscanf(packet,"%d",&tid))
    {
       Logger("Invalid thread id\n");
       gdb_putpacketz("E01");
       return;
    }
    if(0==tid)
    {
      Logger("Invalid thread id\n");
      gdb_putpacketz("E01");
      return;
    }
    Logger("Thread : %d\n",tid);
    if(!Gdb::isThreadAlive(tid))
    {
      gdb_putpacketz("E01");
      return;
    }else
    {
      gdb_putpacketz("OK");
      return;
    }
}

//
