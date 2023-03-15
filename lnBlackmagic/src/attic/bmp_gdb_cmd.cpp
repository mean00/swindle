
#include "lnArduino.h"
#include "bmp_util.h"
#include "bmp_cortex_registers.h"
#include "bmp_gdb_cmd.h"

#if 0
  #define GDB_LOGGER(...) {}
#else
  #define GDB_LOGGER Logger
#endif


bool Gdb::startGatheringSymbol()
{
  allSymbols.clear();
  allSymbols.queryNextSymbol();
  return true;
}

uint32_t Gdb::getCurrentThreadId()
{
  uint32_t pxCurrentTcb;
  if(!allSymbols.readSymbolValue(spxCurrentTCB,pxCurrentTcb))
  {
    return 0;
  }
  uint32_t threadId=readMem32(pxCurrentTcb,O(OFFSET_TASK_NUM)); //68
  return threadId;
}
// Ask the current thread
void Gdb::Qc()
{
  uint32_t threadId=getCurrentThreadId();
  char tst[10+3];

  //if(!threadId) /threadId=2;
  sprintf(tst,"QC%x",threadId);
  gdb_putpacket(tst,strlen(tst));  
  GDB_LOGGER(tst);GDB_LOGGER("\n");
}
//
void Gdb::threadInfo(uint32_t  threadId)
{
  lnThreadInfo *info=threadCache->searchForTid(threadId);
  if(!info)
  {
    gdb_putpacketz("E01");
    return;
  }
  uint32_t tcb=info->tcb;
  if(!tcb) // assuming zero is not a valid address
  {
        gdb_putpacketz("E01");
        return;
  }
  //
  stringWrapper wrapper;
  int maxLen=O(MAX_TASK_NAME_LEN);
  uint32_t name=tcb+O(OFFSET_TASK_NAME);
  char taskName[maxLen+1];
  target_mem_read(cur_target,taskName,name,maxLen);
  taskName[maxLen]=0;
  wrapper.append(taskName);

#if 0
    const char *st;
    switch(sym) // The queue it has been pulled from, gives the task state
    {
      case spxCurrentTCB:           st="R";break;
      case sxSuspendedTaskList:     st="S";break;
      case spxDelayedTaskList:      st="D";break;
      case spxReadyTasksLists:      st="r";break;
      default: st="?";break;
    }
    // Get task name now
    wrapper.append("[");
    wrapper.append(st);
    wrapper.append("]");
#endif
    wrapper.append("  TCB: 0x");
    wrapper.appendHex32(tcb);


    char *in=wrapper.string();;
    int l=strlen(in);
    char out[2*l+1];
    hexify(out,in,l);
    free(in);
    GDB_LOGGER(out);
    gdb_putpacket(out, 2*l);
  }
  //
  bool Gdb::decodeSymbol(int len, const char *packet)
  {
    allSymbols.decodeSymbol(len, packet);
    return true;
  }
  /**
  This takes about 20 ms
  */
  bool Gdb::switchThread(uint32_t  threadId)
  {
    //return false;
    uint32_t currentTcb;
    if(!allSymbols.readSymbolValue(spxCurrentTCB,currentTcb))
    {
      return false;
    }
    uint32_t currentThreadId=readMem32(currentTcb,O(OFFSET_TASK_NUM)); //68
    if(currentThreadId==threadId)
    {
        GDB_LOGGER("Already on the right thread..\n");
        gdb_putpacketz("OK");
        return true;
    }
    // look up the new thread
    lnThreadInfo *info=threadCache->searchForTid(threadId);
    if(!info)
    {
      return false;
    }
    uint32_t tcb=info->tcb;
    if(!tcb) // assuming zero is not a valid address
    {
      return false;
    }

    // Save current thread
    //-----------------------
    Logger("Save\n");
    cortexRegs *regs=createCortexWrite(cur_target);
    regs->loadRegisters();
    uint32_t sp=regs->read(13);
    Logger("Save0\n");
    GDB_LOGGER("Current Thread ID=%d, current TCB=%x sp=%x\n",currentThreadId,currentTcb,sp);
    GDB_LOGGER("PC=%x, SP=%x\n",regs->read(14),sp);
    sp-=regs->stackNeeded(regs->read(14));
    regs->storeRegistersButSpToMemory(sp);
    Logger("Save1\n");
    writeMem32(currentTcb,0,sp); // store sp on the TCB for current stack
    //
    // restore the other thread
    //----------------------------
    Logger("Restore\n");
    sp=readMem32(tcb,0); // top of stack
    GDB_LOGGER("New Thread ID=%d, thread TCB=%x sp=%x\n",threadId,tcb,sp);
    sp=regs->loadRegistersButSpFromMemory(sp);
    Logger("Restore0\n");
    GDB_LOGGER("PC=%x, SP=%x\n",regs->read(14),sp);
    regs->write(13,sp);   // update sp
    regs->setRegisters(); // set actual registers from regs
    Logger("Restore1\n");
    delete regs;
    regs=NULL;
    // update pxcurrentTCB
    uint32_t pcurrentTcb;
    if(!allSymbols.readSymbol(spxCurrentTCB,pcurrentTcb))
    {
      return false;
    }
    GDB_LOGGER("updating pxCurrent TCB at %x with %x\n",pcurrentTcb,tcb);
    writeMem32(pcurrentTcb,0,tcb);
    gdb_putpacketz("OK");
    return true;
  }


bool Gdb::isThreadAlive(uint32_t  threadId)
{
  lnThreadInfo *info=threadCache->searchForTid(threadId);
  if(!info)
  {
    return false;
  }
  if(!info->tcb)
  {
    return false;
  }
  return true;
}

cortexRegs *createCortexWrite(target *t)
{
  const CoreMatching *c=cores;
  const char *name=t->core;
  while(c->core)
  {
    if(!strcmp(name,c->core))
    {
      switch(c->coreId)
      {
          default: Logger("Cannot identify core\n");
          case 0: return new cortexRegsM3;break;
          case 1:
                if(O(MPU_ENABLED)==1)
                  return new cortexRegsM4;
                else
                  return new cortexRegsM3; // it is a M4 core in m3 mode
                break;
      }
    }
    c++;
  }
  Logger("Unknown core %d\n",name);
  return new cortexRegsM3;
}
// EOF
