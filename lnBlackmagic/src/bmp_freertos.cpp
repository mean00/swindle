/*

 */
 #include "lnArduino.h"
 #include "bmp_string.h"
 extern "C"
 {
 #include "hex_utils.h"
}

extern "C" void gdb_putpacket(const char *packet, int size);
//http://web.mit.edu/rhel-doc/3/rhel-gdb-en-3/general-query-packets.html
// https://www.embecosm.com/appnotes/ean4/embecosm-howto-rsp-server-ean4-issue-2.html#id3059472
// qC : current Thread  <=
// H : set thread
// Hg0
// qOffsets
// qSymbol::
// qfThreadInfo
class FosSymbol
{
public:
      uint32_t   address;
      const char *symbolName;
};
#define MAX_FOS_SYMBOLS 10

/**

*/
#define MAX_SYMBOL_LENGTH 50
static const char *neededSymbols[]=
{
    "pxCurrentTCB",
    "xSuspendedTaskList",
    "xDelayedTaskList1",
    "xDelayedTaskList2",
    "pxReadyTasksLists",
    NULL
};
enum FreeRTOSSymbols
{
    pxCurrentTCB=0,
    xSuspendedTaskList=1,
    xDelayedTaskList1=2,
    xDelayedTaskList2=3,
    pxReadyTasksLists=4
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
    Logger("Symbol %s not found\n",name);
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
      gdb_putpacket("E01",3);
      return false;
    }
    // grab the name now, since we passed the sscanf, we know there is a ':'
    const char *name=packet;
    while(*name!=':') name++;
    name++;

    int hexLen=packet+len-name;

    hexLen=((hexLen+1)&~1)/2; // in decoded bytes
    if(hexLen>MAX_SYMBOL_LENGTH) xAssert(0);
    unhexify(_decodedName,name,hexLen);
    Logger("got symbol [%s] with value 0x%x\n",_decodedName,addr);
    add(_decodedName,(uint32_t )addr);
    queryNextSymbol();
    return true;
  }
protected:
  uint32_t  _address[MAX_FOS_SYMBOLS];
  int       _nbSymbols;
  int       _nbNeededSymbols;
  char      _decodedName[MAX_SYMBOL_LENGTH];
};

FosSymbol *currentTCB=NULL;
AllSymbols allSymbols;
/**

*/
void initFreeRTOS()
{
  currentTCB=NULL;
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
//--https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#thread_002did-syntax
// this is thread RTOS ; let's assume the # of threads is not ridiculous and fits
// into one frame
extern "C" void execqfThreadInfo(const char *packet, int len)
{
  Logger("::: qfThreadinfo:%s\n",packet);
  stringWrapper wrapper;
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
