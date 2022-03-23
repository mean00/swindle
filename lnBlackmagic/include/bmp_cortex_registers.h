#pragma once

class cortexRegs
{
public:
    void storeRegistersToMemory(uint32_t adr)
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
    }
    void loadRegistersFromMemory(uint32_t adr)
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
    }
    void loadRegisters()
    {
        target_regs_read(cur_target,_regs);
    }
    void setRegisters()
    {
        target_regs_write(cur_target,_regs);
    }
    uint32_t  read(int reg) {return _regs[reg];}
    void      write(int reg,uint32_t val) { _regs[reg]=val;}

protected:
  uint32_t _regs[17]; //{16} is psr
};
//
