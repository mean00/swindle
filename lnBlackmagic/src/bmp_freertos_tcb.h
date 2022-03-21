#define cannotParse() { return false;}
#define cannotParseVoid() { Logger("error \n");return;}


//--https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#thread_002did-syntax
// this is # of freertos threads; let's assume it is not ridiculouslu big and fits
// into one frame
#define LIST_SIZE 20

#define TCB_LOG(...)  {}

/**
    This is the base class to parse all threads
    overload the execList function to perform a specific action
*/
class ThreadParserBase
{
public:
      ThreadParserBase()
      {

      }
      ~ThreadParserBase()
      {

      }

      //--
      bool parseList2(uint32_t listStart)
      {
          uint32_t nbItem=target_mem_read32(cur_target,listStart); // Nb of items
          TCB_LOG("starting list a  %x \n",listStart);
          TCB_LOG("Found %d items\n",nbItem);
          if(!nbItem) return true; // empty
          uint32_t index=target_mem_read32(cur_target,listStart+4);// List starting point
          TCB_LOG("index  %x \n",index);
          uint32_t cur=index;
          TCB_LOG("scan3 start %x, %d items\n",listStart,nbItem);
          for(int i=0;i<nbItem;i++)
          {
              TCB_LOG("Now at 0x%x\n",cur);
              // Read Owner is actually the TCB
              uint32_t owner=target_mem_read32(cur_target,cur+12);
              if(owner)
              {
                execList(owner);
              }
              uint32_t old=cur;
              cur=target_mem_read32(cur_target,cur+4); // next
              TCB_LOG("Next at 0x%x\n",cur);
              if(cur==old) i=nbItem;
          };
          return true;
      }

      bool parseList(uint32_t listStart)
      {
          uint32_t nbItem=target_mem_read32(cur_target,listStart); // Nb of items
          TCB_LOG("starting list a  %x \n",listStart);
          TCB_LOG("Found %d items\n",nbItem);
          if(!nbItem) return true; // empty
          uint32_t index=target_mem_read32(cur_target,listStart+4);// List starting point
          TCB_LOG("index  %x \n",index);
          uint32_t head=index;
          // go one step away, the first one is dummy
          uint32_t cur=target_mem_read32(cur_target,index+4);// First real entry
          uint32_t start=cur;
          TCB_LOG("scan2 start %x, %d items\n",start,nbItem);
          do
          {
              TCB_LOG("Now at 0x%x\n",cur);
              // Read Owner is actually the TCB
              uint32_t owner=target_mem_read32(cur_target,cur+12);
              if(owner)
              {
                execList(owner);
              }
              uint32_t old=cur;
              cur=target_mem_read32(cur_target,cur+4); // next
              TCB_LOG("Next at 0x%x\n",cur);
              if(cur==old) cur=head;
          }while(cur!=head);
          return true;
      }

      bool parseSymbol(FreeRTOSSymbols symb,bool isPointer)
      {
        uint32_t *pAdr=allSymbols.getSymbol(symb );
        if(!pAdr)
        {
          cannotParse();
        }
        uint32_t adr=*pAdr;
        if(isPointer)
            adr=target_mem_read32(cur_target,adr);
        // Read it
        return parseList(adr);
      }
      bool parseReadyThreads()
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
        for(int prio=0;prio<nbPrio;prio++)
        {
            uint32_t nbItem=target_mem_read32(cur_target,adr); // Nb of items
            if(nbItem)
            {
              parseList2(adr);
            }
            adr+=LIST_SIZE;
        }
        return true;
      }
      void run()
      {
        TCB_LOG("------------------ ready --\n");
        parseReadyThreads();
        TCB_LOG("------------------- delayed --\n");
        parseSymbol(spxDelayedTaskList,true);
        TCB_LOG("------------------- suspended --\n");
        parseSymbol(sxSuspendedTaskList,false);
      }
      virtual void execList(uint32_t tcbAddress)=0;
};
//
