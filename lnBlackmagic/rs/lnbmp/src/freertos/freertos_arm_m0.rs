
use crate::freertos::freertos_trait::freertos_switch_handler;
use crate::bmp::{bmp_read_registers, bmp_write_register};
use crate::bmp::{bmp_read_mem,bmp_read_mem32, bmp_write_mem32};

const STACKED_REGISTER_SIZE : u32 = 64;
const M0_REGISTER_COUNT: usize  = 17;
/**
 * 
 */
pub struct freertos_switch_handler_m0
{
    registers : [u32;M0_REGISTER_COUNT], // R0..R15 + PSR
    pointer   : u32, // pseudo stack
}

impl freertos_switch_handler_m0 {
    pub fn new() -> Self
    {
        freertos_switch_handler_m0 
        {
            registers : [0;M0_REGISTER_COUNT],  // R0..R15 + PSR
            pointer : 0,
        }
    }
    fn push( &mut self, first : u32, last : u32 ) -> bool
    {
        let mut item : [u32;1]=[0];
        for i in first..last {
            item[0]=self.registers[i as usize];        
            if !bmp_write_mem32(self.pointer, &item)   {                
                return false;
            }     
            self.pointer += 4;  
        }
        true
    }
    fn pop( &mut self, first : u32, last : u32 ) -> bool
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


/**
 * 
 */
impl freertos_switch_handler for freertos_switch_handler_m0
{
    /**
     * write internal to actual registers
     */
    fn write_current_registers(&self ) -> bool
    {        
        for i  in 0..M0_REGISTER_COUNT {
            bmp_write_register(i as u32, self.registers[i]);
        }
        true
    }
    /**
     * copy actual registers to internal
     */
    fn read_current_registers(&mut self )->bool
    {
        let regs = bmp_read_registers();
        if regs.len() < M0_REGISTER_COUNT
        {
            return false;
        }
        self.registers[..M0_REGISTER_COUNT].copy_from_slice(&regs[..M0_REGISTER_COUNT]);
        true
    }
    /**
     * write register dump to adr, careful the register are out of order 
     * We write them as if it was a freertos task switch
     */
    fn write_registers_to_stack(&mut self) -> bool
    {
        self.registers[13] -= STACKED_REGISTER_SIZE; // adjust stack to be at the beginning
        self.pointer = self.registers[13];
        // now write the registers onto the stack
        self.push(4,12);   // push  R4 to R12 excluded
        self.push(0,4);    // push  R0 to R4 excluded
        self.push(12,13);  // R12
        self.push(14,15);  // R14 LR
        self.push(15,16);  // R15 PPC
        self.push(16,17);  // R16 PSR        
        true
    }
    /**
     * read register dump from adr, careful the register are out of order 
     */
    fn read_registers_from_addr(&mut self, address : u32)->bool
    {
        self.registers[13] = address + STACKED_REGISTER_SIZE;
        // rewind by 16 *4=64 bytes, we dont save SP on SP but on TCP
        self.pointer=address;
        // now read the registers onto the stack
        self.pop(4,12);    // push  R4 to R12 excluded
        self.pop(0,4);     // push  R0 to R4 excluded
        self.pop(14,15);  // R14 LR
        self.pop(15,16);  // R15 PPC
        self.pop(16,17);  // R16 PSR
        true
    }

    fn get_sp(&self) -> u32
    {
        self.registers[13]
    }

}
// EOF