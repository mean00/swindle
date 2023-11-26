/**
 *  Basic functions for cortex M Core
 * 
 */
use crate::freertos::freertos_trait::freertos_switch_handler;
use crate::bmp::{bmp_read_registers, bmp_write_register};
use crate::bmp::{bmp_read_mem,bmp_read_mem32, bmp_write_mem32};


const CORTEXM_GPR_REG_COUNT: usize  = 17;                   // r00..r15+xpr = 17 reg
/**
 * 
 */
pub struct freertos_cortexm_core
{
    pub registers : [u32;CORTEXM_GPR_REG_COUNT], // R0..R15 + PSR
    pub pointer   : u32, // pseudo stack
}
/**
 * 
 */
impl freertos_cortexm_core
{
    pub fn new() -> Self
    {
        freertos_cortexm_core 
        {
            registers : [0;CORTEXM_GPR_REG_COUNT],  // R0..R15 + PSR
            pointer : 0,
        }
    }
    /**
     * write internal to actual registers
     */
    pub fn write_current_gpr_registers(&self ) -> bool
    {        
        for i  in 0..CORTEXM_GPR_REG_COUNT {
            bmp_write_register(i as u32, self.registers[i]);
        }
        true
    }
    /**
     * copy actual registers to internal
     */
    pub fn read_current_gpr_registers(&mut self )->bool
    {
        let regs = bmp_read_registers();
        if regs.len() < CORTEXM_GPR_REG_COUNT
        {
            return false;
        }
        self.registers[..CORTEXM_GPR_REG_COUNT].copy_from_slice(&regs[..CORTEXM_GPR_REG_COUNT]);
        true
    }
    /**
     * 
     */
    pub fn get_sp(&self) -> u32
    {
        self.registers[13]
    }
    /**
     * 
     */
    pub fn push( &mut self, first : u32, last : u32 ) -> bool
    {
        let to_push = last-first;
        self.pointer -= 4*to_push;  
        let mut current = self.pointer;
        let mut item : [u32;1]=[0];
        for i in first..last {
            item[0]=self.registers[i as usize];        
            if !bmp_write_mem32(current, &item)   {                
                return false;
            }     
            current += 4;  
        }
        true
    }
    /**
     * 
     */
    pub fn pop( &mut self, first : u32, last : u32 ) -> bool
    {
        let mut item : [u32;1]=[0];
        for i in first..last {
            
            if !bmp_read_mem32(self.pointer, &mut item)   {                
                return false;
            }     
            self.registers[i as usize]=item[0];
            self.pointer += 4;  
        }
        true
    }


}
//--