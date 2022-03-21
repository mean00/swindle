/*
https://ftp.gnu.org/old-gnu/Manuals/gdb/html_chapter/gdb_15.html
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
#include "bmp_symbols.h"
AllSymbols allSymbols;
/**

*/
void initFreeRTOS()
{

  allSymbols.clear();
}
//
extern "C" void execqOffsets(const char *packet, int len)
{
  // it's xip...
  gdb_putpacket("Text=0;Data=0;Bss=0", 19); // 7 7 5=>19
}
/*
    Grab FreeRTOS symbols
*/
extern "C" void execqSymbol(const char *packet, int len)
{
  Logger("<execqSymbol>:<%s>\n",packet);
  if(len==1 && packet[0]==':') // :: : ready to serve https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
  {
    allSymbols.clear();
    allSymbols.queryNextSymbol();
    return;
  }
  if(len>1)
  {
      allSymbols.decodeSymbol(len,packet);
      return;
  }
  gdb_putpacket("OK", 2);
}
/**

*/
extern "C" void execqThreadInfo(const char *packet, int len)
{
  Logger("*** execqThreadInfo:%s\n",packet);
  gdb_putpacketz("");
}
#include "bmp_freertos_tcb.h"
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
    void execList(uint32_t tcbAdr)
    {
        uint32_t id=target_mem_read32(cur_target,tcbAdr+allSymbols._debugInfo.OFFSET_TASK_NUM);
        if(strlen(_w->string()))
          _w->append(",");
        _w->appendHex64(id);
    }
protected:
  stringWrapper *_w;
};

extern "C" void execqfThreadInfo(const char *packet, int len)
{
  uint32_t *pAdr;
  uint32_t adr;
  Logger("::: qfThreadinfo:%s\n",packet);
  if(!cur_target)
  {
    gdb_putpacketz("");
    return;
  }
  if(!allSymbols.readDebugBlock())
  {
    gdb_putpacketz("");
    return;
  }
  if(allSymbols._debugInfo.MAGIC!=LN_FREERTOS_MAGIC)
  {
    gdb_putpacketz("");
    return;
  }

  stringWrapper wrapper;
  listThread list(&wrapper);
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
extern "C" void execqsThreadInfo(const char *packet, int len)
{
  Logger("::: qsThreadinfo:%s\n",packet);
  gdb_putpacket("l", 1);
}

extern "C" void exect_qThreadExtraInfo(const char *packet, int len)
{
  Logger("::: exect_qThreadExtraInfo:%s\n",packet);
  const char *hey="Hey you!";
  int l=strlen(hey);
  char out[2*l+1];
  hexify(out,hey,l);
  Logger(out);
  gdb_putpacket(out, 2*l);


}

extern "C" void exect_qC(const char *packet, int len)
{
  Logger("::: exect_qC:%s\n",packet);
  if(allSymbols._debugInfo.MAGIC!=LN_FREERTOS_MAGIC)
  {
    gdb_putpacketz("");
    return;
  }
  uint32_t *pxCurrentTcb=allSymbols.getSymbol(spxCurrentTCB);
  if(!pxCurrentTcb)
  {
    gdb_putpacketz("");
    return;
  }
  uint32_t tcb=target_mem_read32(cur_target,*pxCurrentTcb);
  Logger("Current TCB=%x\n",tcb);

  uint32_t tid_adr=tcb+68;
  Logger("Current TID ADR=%x\n",tid_adr);
  uint32_t threadId=target_mem_read32(cur_target,tid_adr);
  Logger("Current TID =%x\n",threadId);

  stringWrapper wrapper;
  if(!threadId) threadId=2;
  wrapper.appendHex32(threadId);
  char *out=wrapper.string();
  gdb_putpacket2("QC",2,out,strlen(out));
  Logger(out);
  free(out);
}

/**

*/
extern "C" bool lnProcessCommand(int size, const char *data)
{
    Logger("Received packet %s\n",data);
    return false;
}

//
