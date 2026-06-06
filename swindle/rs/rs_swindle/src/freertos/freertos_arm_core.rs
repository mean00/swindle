setup_log!(false);
//use crate::bmpwarning;
crate::gdb_print_init!();

use crate::bmp::{bmp_read_mem32, bmp_write_mem32};
use crate::bmp::{bmp_read_registers, bmp_write_register};
/*
 *  Basic functions for cortex M Core
 *
 */
const CORTEXM_GPR_REG_COUNT: usize = 17; // r00..r15+xpr = 17 reg

/*
 *
 */
pub struct freertos_cortexm_core {
    pub registers: [u32; CORTEXM_GPR_REG_COUNT], // R0..R15 + PSR
    pub pointer: u32,                            // pseudo stack
}
/*
 *
 */
impl freertos_cortexm_core {
    pub fn new() -> Self {
        freertos_cortexm_core {
            registers: [0; CORTEXM_GPR_REG_COUNT], // R0..R15 + PSR
            pointer: 0,
        }
    }
    /*
     * write internal to actual registers
     */
    pub fn write_current_gpr_registers(&self) -> bool {
        for i in 0..CORTEXM_GPR_REG_COUNT {
            if !bmp_write_register(i as u32, self.registers[i]) {
                bmpwarning!("write_current_gpr_registers: failed to write register {}\n", i);
                return false;
            }
        }
        true
    }
    /*
     * copy actual registers to internal
     */
    pub fn read_current_gpr_registers(&mut self) -> bool {
        let regs = bmp_read_registers();
        if regs.len() < CORTEXM_GPR_REG_COUNT {
            bmpwarning!("Registers list too small\n");
            return false;
        }
        self.registers[..CORTEXM_GPR_REG_COUNT].copy_from_slice(&regs[..CORTEXM_GPR_REG_COUNT]);
        true
    }
    /*
     *
     */
    pub fn get_sp(&self) -> u32 {
        self.registers[13]
    }
    /*
     *
     */
    /// Write a single value to the stack, decrementing pointer first.
    /// Used for values not in the register array (e.g. EXC_RETURN, PSPLIM).
    pub fn write_ascending(&mut self, value: u32) -> bool {
        self.pointer -= 4;
        bmp_write_mem32(self.pointer, &[value])
    }

    pub fn push_ascending(&mut self, first: usize, last: usize) -> bool {
        let to_push = last - first;
        self.pointer -= 4u32 * to_push as u32;
        bmp_write_mem32(self.pointer, &self.registers[first..last])
    }
    /*
     *
     */
    pub fn push_descending(&mut self, first: usize, last: usize) -> bool {
        let to_push = last - first;
        self.pointer -= 4u32 * to_push as u32;
        for reg in 0..to_push {
            let r = last - 1 - reg;
            bmp_write_mem32(
                self.pointer + 4u32 * (reg as u32),
                &self.registers[r..r + 1],
            );
        }
        true
    }
    /*
     *
     */
    pub fn pop_ascending(&mut self, first: usize, last: usize) -> bool {
        let ret = bmp_read_mem32(self.pointer, &mut self.registers[first..last]);
        self.pointer += 4 * (last - first) as u32;
        ret
    }
    /*
     *
     */
    pub fn pop_descending(&mut self, first: usize, last: usize) -> bool {
        let to_push = last - first;
        let mut ret: bool = true;
        for reg in 0..to_push {
            let r = last - 1 - reg;
            ret &= bmp_read_mem32(
                self.pointer + 4u32 * (reg as u32),
                &mut self.registers[r..r + 1],
            );
        }
        self.pointer += 4 * (last - first) as u32;
        ret
    }
}
//--
