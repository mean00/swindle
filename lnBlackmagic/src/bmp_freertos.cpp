/*

 */
 #include "lnArduino.h"
 #include "bmp_string.h"

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
static const char *neededSymbols[]={"pxCurrentTCB",NULL};
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
      _currentSymbolQuery=0;
      _nbNeededSymbols=(sizeof(neededSymbols)/sizeof(const char *))-1;
  }
  void add(const char *name)
  {
    _symbols[_nbSymbols++].symbolName=name;
    _symbols[_nbSymbols++].address=0;
    xAssert(_nbSymbols<=MAX_FOS_SYMBOLS);
  }
  FosSymbol *getSymbol(const char *name)
  {
    for(int i=0;i<_nbSymbols;i++)
    {
      if(!strcmp(name,_symbols[i].symbolName)) return _symbols+i;
    }
    Logger("Symbol %s not found\n",name);
    return NULL;
  }
  bool queryNextSymbol()
  {
    if(_currentSymbolQuery==_nbNeededSymbols)
        return false;
    stringWrapper wr;
    wr.append("qSymbol:");
    wr.appendHexified(neededSymbols[0]);
    char *s=wr.string();
    gdb_putpacket(s, strlen(s));
    free(s);
    return true;
  }
  bool decodeSymbol(int len,const char *packet)
  {
    return true;
  }
protected:
  FosSymbol _symbols[MAX_FOS_SYMBOLS];
  int       _nbSymbols;
  int       _nbNeededSymbols;
  int       _currentSymbolQuery;
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
//
extern "C" void execqSymbol(const char *packet, int len)
{
  Logger("<execqSymbol>:<%s>\n",packet);
  if(len==1 && packet[0]==':') // :: : ready to serve https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
  {
    if(!allSymbols.queryNextSymbol())
        gdb_putpacket("OK", 2);
    return;
  }
  if(len>1)
  {
      allSymbols.decodeSymbol(len,packet);
  }
  gdb_putpacket("OK", 2);
}
//
extern "C" void execqThreadInfo(const char *packet, int len)
{
  Logger("*** execqThreadInfo:%s\n",packet);
  gdb_putpacket("", 0);
}
//--
extern "C" void execqfThreadInfo(const char *packet, int len)
{
  Logger("*** qfThreadinfo:%s\n",packet);
  gdb_putpacket("m", 1);
}


/**

*/
extern "C" bool lnProcessCommand(int size, const char *data)
{
    Logger("Received packet %s\n",data);
    return false;
}
//
