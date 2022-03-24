#define cannotParse() { return false;}
#define cannotParseVoid() { Logger("error \n");return;}


//--https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#thread_002did-syntax

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
      bool parseReadyList( uint32_t listStart)
      {
          uint32_t nbItem=readMem32(listStart,O(OFFSET_LIST_NUMBER_OF_ITEM)); // 0 Nb of items
          TCB_LOG("starting list a  %x \n",listStart);
          TCB_LOG("Found %d items\n",nbItem);
          if(!nbItem) return true; // empty
          uint32_t index=readMem32(listStart,O(OFFSET_LIST_INDEX));// List starting point
          TCB_LOG("index  %x \n",index);
          uint32_t cur=index;
          TCB_LOG("scan3 start %x, %d items\n",listStart,nbItem);
          for(int i=0;i<nbItem;i++)
          {
              TCB_LOG("Now at 0x%x\n",cur);
              // Read Owner is actually the TCB
              uint32_t owner=readMem32(cur,O(OFFSET_LIST_ITEM_OWNER)); //12
              if(owner)
              {
                if(!execList(spxReadyTasksLists, owner)) return true;
              }
              uint32_t old=cur;
              cur=readMem32(cur,O(OFFSET_LIST_ITEM_NEXT)); // 4 next
              TCB_LOG("Next at 0x%x\n",cur);
              if(cur==old) i=nbItem;
          };
          return true;
      }

      bool parseList(FreeRTOSSymbols symb, uint32_t listStart)
      {
          uint32_t nbItem=readMem32(listStart,O(OFFSET_LIST_NUMBER_OF_ITEM)); // 0 Nb of items
          TCB_LOG("starting list a  %x \n",listStart);
          TCB_LOG("Found %d items\n",nbItem);
          if(!nbItem) return true; // empty
          uint32_t index=readMem32(listStart,O(OFFSET_LIST_INDEX));// 4List starting point
          TCB_LOG("index  %x \n",index);
          uint32_t head=index;
          // go one step away, the first one is dummy
#warning FIXME
          uint32_t cur=readMem32(index,O(OFFSET_LIST_ITEM_NEXT));// 4First real entry
          if(cur)
          {
            uint32_t start=cur;
            TCB_LOG("scan2 start %x, %d items\n",start,nbItem);
            do
            {
                TCB_LOG("Now at 0x%x\n",cur);
                // Read Owner is actually the TCB
                uint32_t owner=readMem32(cur,O(OFFSET_LIST_ITEM_OWNER)); // 12
                if(owner)
                {
                  if(!execList(symb,owner)) return false;
                }
                uint32_t old=cur;
                cur=readMem32(cur,O(OFFSET_LIST_ITEM_NEXT)); // 4 next
                TCB_LOG("Next at 0x%x\n",cur);
                if(cur==old) cur=head;
              }while(cur!=head && cur);
          }
          return true;
      }

      bool parseSymbolList(FreeRTOSSymbols symb,bool isPointer)
      {
        uint32_t adr;
        bool r;
        if(!isPointer)
        {
            r=allSymbols.readSymbol(symb,adr);
        }else
        {
            r=allSymbols.readSymbolValue(symb,adr);
        }
        if(!r)
        {
          return true;
        }
        return parseList(symb,adr);
      }
      bool parseReadyThreads()
      {
        uint32_t adr;
        if(!allSymbols.readSymbol(spxReadyTasksLists,adr))
        {
          return true;
        }
        // Read the number of tasks
        int nbPrio=O(NB_OF_PRIORITIES);
        int listSize=O(LIST_SIZE);
        if(!nbPrio)
        {
          cannotParse();
        }
        for(int prio=0;prio<nbPrio;prio++)
        {
            uint32_t nbItem=readMem32(adr,O(OFFSET_LIST_NUMBER_OF_ITEM)); // Nb of items
            if(nbItem)
            {
              if(!parseReadyList(adr)) return false;
            }
            adr+=listSize;
        }
        return true;
      }
      void run()
      {
        TCB_LOG("------------------ ready --\n");
        if(!parseReadyThreads()) return;
        TCB_LOG("------------------- delayed --\n");
        if(!parseSymbolList(spxDelayedTaskList,true)) return;
        TCB_LOG("------------------- suspended --\n");
        if(!parseSymbolList(sxSuspendedTaskList,false)) return;
      }
      // return false if we stop here
      virtual bool execList(FreeRTOSSymbols symb, uint32_t tcbAddress)=0;
};
//
