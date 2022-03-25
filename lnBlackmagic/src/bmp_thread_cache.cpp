/*
  Small cache to store TCB address, thread ID and which queue it belongs to.
*/
#include "bmp_util.h"
#include "bmp_thread_cache.h"
#include "bmp_gdb_cmd.h"


class listThread : public ThreadParserBase
{
public:
    listThread()
    {
    }
    bool execList(FreeRTOSSymbols state,uint32_t tcbAdr)
    {
        uint32_t id=readMem32(tcbAdr,O(OFFSET_TASK_NUM));
        threadCache->add(id,tcbAdr); //,state);
        return true;
    }
};

void lnThreadInfoCache::clear()
{
    lnThreadInfo *h=_q;
    _q=NULL;
    while(h)
    {
        lnThreadInfo *me=h;
        h=h->next;
        delete me;
        me=NULL;
    }
}
void lnThreadInfoCache::add(uint32_t tid,uint32_t tcb)
{
  lnThreadInfo *e=new lnThreadInfo(tid,tcb);
  e->next=_q;
  _q=e;
}
lnThreadInfo *lnThreadInfoCache::searchForTid(uint32_t id)
{
  lnThreadInfo *e=_q;
  while(e)
  {
      if(e->id==id) return e;
      e=e->next;
  }
  return NULL;
}
uint32_t searchForTcb(uint32_t tcb);
void     lnThreadInfoCache::collectIdAsWrapperString(stringWrapper &w)
{
  lnThreadInfo *e=_q;

  // Grab current thread, make sure it is the 1st
  uint32_t current=Gdb::getCurrentThreadId();
  if(current)
  {
    lnThreadInfo *e=searchForTid(current);
    if(e)
    {
      w.appendHex32(0); // output is hex64
      w.appendHex32(e->id);
      Logger("Thread list %x\n",e->id);
    }
  }
  while(e)
  {
    if(e->id!=current)
    {
      if(strlen(w.string()))
          w.append(",");
      w.appendHex32(0); // output is hex64
      w.appendHex32(e->id);
      Logger("Thread list %x\n",e->id);
    }
    e=e->next;
  }
}


void lnThreadInfoCache::updateCache()
{
  clear();
  listThread list; // list all the threads
  list.run();
}

// EOF
