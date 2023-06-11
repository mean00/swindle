/*
  Small cache to store TCB address, thread ID and which queue it belongs to.
*/
#pragma once
struct lnThreadInfo
{
  public:
    lnThreadInfo(uint32_t nid, uint32_t ntcb)
    {
        id = nid;
        tcb = ntcb;
        next = NULL;
    }
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
        _q = NULL;
    }
    ~lnThreadInfoCache()
    {
        clear();
    }
    void clear();
    void updateCache();
    void add(uint32_t tid, uint32_t tcb);
    lnThreadInfo *searchForTid(uint32_t id);
    uint32_t searchForTcb(uint32_t tcb);
    void collectIdAsWrapperString(stringWrapper &w);

  protected:
    lnThreadInfo *_q;
};
