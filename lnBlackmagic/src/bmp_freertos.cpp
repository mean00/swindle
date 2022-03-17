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

//http://web.mit.edu/rhel-doc/3/rhel-gdb-en-3/general-query-packets.html
// https://www.embecosm.com/appnotes/ean4/embecosm-howto-rsp-server-ean4-issue-2.html#id3059472
// qC : current Thread  <=
// H : set thread
// Hg0
// qOffsets
// qSymbol::
// qfThreadInfo
#define MAX_FOS_SYMBOLS 10



/**

*/
#define MAX_SYMBOL_LENGTH 50
static const char *neededSymbols[]=
{
    "pxCurrentTCB",
    "xSuspendedTaskList",
    "pxDelayedTaskList",
    "pxReadyTasksLists",
    "uxCurrentNumberOfTasks",
    "freeRTOSDebug",
    NULL
};
enum FreeRTOSSymbols
{
    pxCurrentTCB=0,
    xSuspendedTaskList=1,
    xDelayedTaskList=2,
    pxReadyTasksLists=3,
    uxCurrentNumberOfTasks=4,
    freeRTOSDebug=5
};

/**

*/
class AllSymbols
{
public:

  AllSymbols()
  {
      clear();
  }
  void clear()
  {
      _nbSymbols=0;
      _nbNeededSymbols=(sizeof(neededSymbols)/sizeof(const char *))-1;
      memset(&_debugInfo,0,sizeof(_debugInfo));
      for(int i=0;i< MAX_FOS_SYMBOLS;i++) _address[i]=-1;
  }
  /**

  */
  bool readDebugBlock()
  {

    if(!cur_target)
    {
      gdb_putpacketz("");
      return false;
    }
    // do we have the debug block ?
    uint32_t *debugBlock=getSymbol(freeRTOSDebug);
    if(!debugBlock)
    {
      gdb_putpacketz("");
      return false;
    }
    if(!*debugBlock)
    {
      gdb_putpacketz("");
      return false;
    }
    // read info block

  #define READ_FIELD(field)   _debugInfo.field=target_mem_read32(cur_target,*debugBlock+offsetof(lnFreeRTOSDebug,field));
    READ_FIELD(MAGIC)
    READ_FIELD(NB_OF_PRIORITIES)
    READ_FIELD(MPU_ENABLED)
    READ_FIELD(MAX_TASK_NAME_LEN)
    if(_debugInfo.MAGIC==LN_FREERTOS_MAGIC)
      return true;
    return false;
  }
  /*
  */
  void add(const char *name,uint32_t  value)
  {
    if(strcmp(name,neededSymbols[_nbSymbols])) xAssert(0);
    _address[_nbSymbols]=value;
    _nbSymbols++;
    xAssert(_nbSymbols<=MAX_FOS_SYMBOLS);
  }
  /*
  */
  uint32_t *getSymbol(const char *name)
  {
    for(int i=0;i<_nbSymbols;i++)
    {
      if(!strcmp(name,neededSymbols[i]))
      {
        if(_address[i]==-1) return NULL;
        return _address+i;
      }
    }
    fdebug2("Symbol %s not found\n",name);
    return NULL;
  }
  uint32_t *getSymbol(FreeRTOSSymbols s)
  {
    if(_address[s]==-1) return NULL;
        return _address+s;
    fdebug2("Symbol %s not found\n",s);
    return NULL;
  }
  /*
  */
  bool queryNextSymbol()
  {
    if(_nbSymbols==_nbNeededSymbols) // got them all
    {
        gdb_putpacket("OK",2);
        return false;
    }
    stringWrapper wr;
    wr.append("qSymbol:");
    wr.appendHexified(neededSymbols[_nbSymbols]);
    char *s=wr.string();
    gdb_putpacket(s, strlen(s));
    free(s);
    return true;
  }
  /*
  */
  bool decodeSymbol(int len,const char *packet)
  {
    // the format is adress: hexified symbol
    uint32_t addr;

    if(sscanf(packet, "%lx:", &addr)!=1)
    {
      // ok we dont have an address, do we have a name ?
      if(len<3 || packet[0]!=':')
      {
        gdb_putpacket("E01",3);
        return false;
      }
      // use unknown address
      fdebug2("Non existing symbol ! =>");
      addr=-1;
    }
    // grab the name now, since we passed the sscanf, we know there is a ':'
    const char *name=packet;
    while(*name!=':') name++;
    name++;

    int hexLen=packet+len-name;

    hexLen=((hexLen+1)&~1)/2; // in decoded bytes
    if(hexLen>MAX_SYMBOL_LENGTH) xAssert(0);
    unhexify(_decodedName,name,hexLen);
    fdebug2("got symbol [%s] with value 0x%x\n",_decodedName,addr);
    add(_decodedName,(uint32_t )addr);
    queryNextSymbol();
    return true;
  }
protected:
  uint32_t  _address[MAX_FOS_SYMBOLS];
  int       _nbSymbols;
  int       _nbNeededSymbols;
  // We allocate it here statically to avoid stressing the heap on the fly
  char      _decodedName[MAX_SYMBOL_LENGTH];
public:
  lnFreeRTOSDebug _debugInfo;
};


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
// this is thread RTOS ; let's assume the # of threads is not ridiculouslu big and fits
// into one frame
#define LIST_SIZE 20
bool parseReadyThreads(stringWrapper &w)
{

  // we assume is is followed by uxCurrentNumberOfTasks
  uint32_t *pAdr=allSymbols.getSymbol(pxReadyTasksLists);
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
  for(int prio=0;prio<nbPrio;prio++)
  {
      uint32_t nbItem=target_mem_read32(cur_target,adr); // Nb of items
      uint32_t listAdr=adr+4; // list head
      Logger("%d items ready at priority %d\n",nbItem,prio);
      for( int i=0;i<nbItem;i++)
      {


      }
      adr+=LIST_SIZE;
  }

  w.append("l");
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
  if(!parseReadyThreads(wrapper))
  {
    gdb_putpacketz("");
    return;
  }
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
