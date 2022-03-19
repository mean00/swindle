/*

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
  gdb_putpacket("", 0);
}
#define cannotParse() { return false;}


//--https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#thread_002did-syntax
// this is # of freertos threads; let's assume it is not ridiculouslu big and fits
// into one frame
#define LIST_SIZE 20


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

bool parseListAnon(uint32_t listStart)
{
    uint32_t nbItem=target_mem_read32(cur_target,listStart); // Nb of items
    uint32_t index=target_mem_read32(cur_target,listStart+4);// List starting point
    uint32_t cur=target_mem_read32(cur_target,index+4);// First real entry
    uint32_t start=cur;
    Logger("scan start %x\n",start);
    do
    {
        // Read Owner is actually the TCB
        uint32_t  owner=target_mem_read32(cur_target,cur+12);
        uint32_t id=target_mem_read32(cur_target,owner+allSymbols._debugInfo.OFFSET_TASK_NUM);
        Logger(" Found task <%d> TCB=0x%x\n",id,owner);
        cur=target_mem_read32(cur_target,cur+4); // next
        Logger("now at  start %x\n",cur);
    }while(cur!=start);
    return true;
}

bool parseList(stringWrapper &w,FreeRTOSSymbols symbol)
{
  uint32_t *pAdr=allSymbols.getSymbol(symbol );
  if(!pAdr)
  {
    cannotParse();
  }
  uint32_t adr=*pAdr;
  parseListAnon(adr);

  return true;
}


bool parseReadyThreads(stringWrapper &w)
{
  uint32_t *pAdr=allSymbols.getSymbol(spxReadyTasksLists);
  if(!pAdr)
  {
    cannotParse();
  }
  uint32_t adr=*pAdr;
  // Read the number of tasks
  int nbPrio=allSymbols._debugInfo.NB_OF_PRIORITIES;
  if(!nbPrio)
  {
    cannotParse();
  }
  // it's a list of list
  
  for(int prio=0;prio<nbPrio;prio++)
  {
      uint32_t nbItem=target_mem_read32(cur_target,adr); // Nb of items
      uint32_t listAdr=adr+4; // list head
      parseListAnon(listAdr);
      adr+=LIST_SIZE;
  }
  return true;
}

extern "C" void execqfThreadInfo(const char *packet, int len)
{
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
  //parseReadyThreads(wrapper);
  parseList(wrapper,spxDelayedTaskList);
  parseList(wrapper,sxSuspendedTaskList);
  // Grab all the threads in one big array
  gdb_putpacket("m 0", 3);
}
extern "C" void execqsThreadInfo(const char *packet, int len)
{
  Logger("::: qsThreadinfo:%s\n",packet);
  gdb_putpacket("l", 1);
}


/**

*/
extern "C" bool lnProcessCommand(int size, const char *data)
{
    Logger("Received packet %s\n",data);
    return false;
}
//
