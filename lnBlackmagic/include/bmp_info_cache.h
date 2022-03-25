/*
  Small cache to store TCB address, thread ID and which queue it belongs to.
*/
#pragma once
struct lnThreadInfo
{
public:
      uint32_t id;
      uint32_t tcb;
      lnThreadInfo *next;
};
/**

*/
class lnThreadInfoCache
{
public:
      lnThreadInfoCache()
      {
        _q=NULL;
      }
      ~lnThreadInfoCache()
      {
        clear();
      }
      void clear()
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
      void add(uint32_t tid,uint32_t tcb)
      {
        lnThreadInfo *e=new lnThreadInfo();
        e->id=tid;
        e->tcb=tcb;
        e->next=_q;
        _q=e;
      }
      lnThreadInfo *searchForTid(uint32_t id)
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
      void     collectIdAsWrapperString(stringWrapper &w)
      {
        lnThreadInfo *e=_q;
        while(e)
        {
          if(strlen(w.string()))
              w.append(",");
          w.appendHex32(0); // output is hex64
          w.appendHex32(e->id);
          e=e->next;
        }
      }
protected:
    lnThreadInfo *_q;
};
