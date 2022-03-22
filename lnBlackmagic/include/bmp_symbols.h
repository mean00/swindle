
#pragma once

/**

*/
#define MAX_SYMBOL_LENGTH   50
#define MAX_FOS_SYMBOLS     10
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
    spxCurrentTCB=0,
    sxSuspendedTaskList=1,
    spxDelayedTaskList=2,
    spxReadyTasksLists=3,
    suxCurrentNumberOfTasks=4,
    sfreeRTOSDebug=5

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
    uint32_t *debugBlock=getSymbol(sfreeRTOSDebug);
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

  #define READ_FIELD(field)   _debugInfo.field=readMem32(*debugBlock,offsetof(lnFreeRTOSDebug,field));
    READ_FIELD(MAGIC)
    READ_FIELD(NB_OF_PRIORITIES)
    READ_FIELD(MPU_ENABLED)
    READ_FIELD(MAX_TASK_NAME_LEN)
    READ_FIELD(OFFSET_TASK_NUM)

    READ_FIELD(OFFSET_LIST_ITEM_NEXT);
    READ_FIELD(OFFSET_LIST_ITEM_OWNER);

    READ_FIELD(OFFSET_LIST_NUMBER_OF_ITEM);
    READ_FIELD(OFFSET_LIST_INDEX);


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

    if(sscanf(packet, "%x:", &addr)!=1)
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


//
