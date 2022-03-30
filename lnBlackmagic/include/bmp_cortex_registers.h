#pragma once

/*
/!\ Cortex M0/0+/M3 support should work well
The cortex M4/M7 support of FPU is incomplete & will cause problems
The issue is the FreeRTOS scheduler is aware of the use of FPU through the EXC_RETURN / LR bit 4
we do the same here , bit it's wrong. We should look at the system register.

/!\
  Careful : this is a thumb2 representation of registers
  0..12 = R0..R12
  13 = (p)sp
  14 = LR
  15 = PC
  16 = PSR
---
20..32 FPU register
32 FPU PSR
  It is NOT a 1:1 mapping with blackmagic registers
*/

class cortexRegs
{
public:
        virtual uint32_t  storeRegistersButSpToMemory(uint32_t adr)=0;
        virtual uint32_t  loadRegistersButSpFromMemory(uint32_t adr)=0;
        virtual uint32_t  stackNeeded(uint32_t lr)=0;
        virtual void      loadRegisters()=0;
        virtual void      setRegisters()=0;
        virtual uint32_t  read(int reg)=0;
        virtual void      write(int reg,uint32_t val)=0;
protected:
        uint32_t _regs[20+33]; // Arm regs + FPU, only M0,M3,M4!

};
/**

*/
class cortexRegsM3 : public cortexRegs
{
public:
    uint32_t  stackNeeded(uint32_t lr)
    {
      return 4*(8+4+4);
    }

    uint32_t storeRegistersButSpToMemory(uint32_t adr)
    {
      for(int i=0;i<8;i++) // r4..r11
        writeMem32(adr,i*4,_regs[4+i]);
      adr+=8*4;
      for(int i=0;i<4;i++) // r0..r3
        writeMem32(adr,i*4,_regs[i]);
      adr+=4*4;
      writeMem32(adr,0*4,_regs[12]);
      writeMem32(adr,1*4,_regs[14]);
      writeMem32(adr,2*4,_regs[15]);
      writeMem32(adr,3*4,_regs[16]); // psr
      adr+=4*4;
      return adr;
    }
    uint32_t loadRegistersButSpFromMemory(uint32_t adr)
    {
      for(int i=0;i<8;i++) // r4..r11
        _regs[4+i]=readMem32(adr,i*4);
      adr+=8*4;
      for(int i=0;i<4;i++) // r0..r3
        _regs[i]=readMem32(adr,i*4);
      adr+=4*4;
      _regs[12]=readMem32(adr,0*4);
      _regs[14]=readMem32(adr,1*4);
      _regs[15]=readMem32(adr,2*4);
      _regs[16]=readMem32(adr,3*4); // psr
      adr+=4*4;
      return adr;
    }
    void loadRegisters()
    {
        target_regs_read(cur_target,_regs);
        _regs[13]=_regs[18];
    }
    void setRegisters()
    {
        _regs[18]=_regs[13];
        target_regs_write(cur_target,_regs);
    }
    uint32_t  read(int reg)
    {
        return _regs[reg];
    }
    void      write(int reg,uint32_t val)
    {
        _regs[reg]=val;
     }
};

/**

*/
#define BM_FP_OFFSET 20
#define FP_IN_USE(lr) (!(lr & 0x10))

/**
  This is  incorrect regarding LR handling
*/
class cortexRegsM4 : public cortexRegs
{
public:
    uint32_t  stackNeeded(uint32_t lr)
    {
      uint32_t base= 4*(8+1+4+4);
      if(lr) base+=4*(33);
      return base;
    }
    uint32_t storeRegistersButSpToMemory(uint32_t adr)
    {
      for(int i=0;i<8;i++) // r4..r11
          writeMem32(adr,i*4,_regs[4+i]);
      adr+=8*4;
      uint32_t lr=_regs[14];
      writeMem32(adr,8*4,lr);
      adr+=4;

      // fpu ?
      if(FP_IN_USE(lr))
      {
        // store FP 16..31
        int start=BM_FP_OFFSET+1+16;
        for(int i=0;i<16;i++)
            writeMem32(adr,i*4,_regs[start+i]);
        adr+=16*4;
      }

      for(int i=0;i<4;i++) // r0..r3
        writeMem32(adr,i*4,_regs[i]);
      adr+=4*4;
      writeMem32(adr,0*4,_regs[12]);
      writeMem32(adr,1*4,_regs[14]);
      writeMem32(adr,2*4,_regs[15]);
      writeMem32(adr,3*4,_regs[16]); // psr

      adr+=4*4;
      if(FP_IN_USE(lr))
      {
        int start=BM_FP_OFFSET;
        for(int i=0;i<16;i++)
            writeMem32(adr,i*4,_regs[start+1+i]); // S0..s15
        adr+=16*4;
        writeMem32(adr,0*4,_regs[start]); // fpsr
        adr+=1*4;
        adr+=1*4; // reserved

      }
      return adr;
    }
    uint32_t loadRegistersButSpFromMemory(uint32_t adr)
    {
      bool fpu=false;
      for(int i=0;i<8;i++) // r4..r11
          _regs[4+i]=readMem32(adr,i*4);
      adr+=8*4;
      uint32_t lr;
      lr=readMem32(adr,0);
      _regs[14]=lr;
      adr+=1*4;
      // are the FPU registers there ?
      if(FP_IN_USE(lr))
      {
         // fpu !
         fpu=true;
        // store FP 16..31
        int start=BM_FP_OFFSET+1+16;
        for(int i=0;i<16;i++)
            _regs[start+i]=readMem32(adr,i*4);
        adr+=16*4;
      }
      for(int i=0;i<4;i++) // r0..r3
        _regs[i]=readMem32(adr,i*4);
      adr+=4*4;
      _regs[12]=readMem32(adr,0*4);
      _regs[14]=readMem32(adr,1*4);
      _regs[15]=readMem32(adr,2*4);
      _regs[16]=readMem32(adr,3*4); // psr
      adr+=4*4;

      if(FP_IN_USE(lr))
      {
        int start=BM_FP_OFFSET;
        for(int i=0;i<16;i++)
            _regs[start+1+i]=readMem32(adr,i*4); // S0..s15
        adr+=16*4;
        _regs[start]=readMem32(adr,0*4); // fpsr
        adr+=1*4;
        adr+=1*4; // reserved
      }
      return adr;
    }
    void loadRegisters()
    {
        target_regs_read(cur_target,_regs);
        _regs[13]=_regs[18];
    }
    void setRegisters()
    {
        _regs[18]=_regs[13];
        target_regs_write(cur_target,_regs);
    }
    uint32_t  read(int reg)
    {
        return _regs[reg];
    }
    void      write(int reg,uint32_t val)
    {
        _regs[reg]=val;
     }

};

typedef struct CoreMatching
{
  const char *core;
  uint8_t    coreId;
};

static const CoreMatching cores[]={
    {"M0",0},
    {"M0+",0},
    {"M3",0},
    {"M4",1},
    {"M7",1},
    {NULL,NULL},
};
extern cortexRegs *createCortexWrite(target *t);
//
