/**
 * \brief this is basic rv32imac style freertos support for riscv 32 bits
 * It does not support float or rv64

 Mapping between x1...x31 and usage name (s0, ...)    can be found here
    https://en.wikichip.org/wiki/risc-v/registers
*/
use crate::bmp::{bmp_read_mem, bmp_read_mem32, bmp_write_mem32};
use crate::bmp::{bmp_read_registers, bmp_write_register};

crate::setup_log!(false);
/**
 *
 */
use crate::freertos::freertos_trait::freertos_switch_handler;
use crate::{bmplog, bmpwarning};

const RV32_GPRS_REGISTER: usize = 28;
const RV32_TOP_REGISTER: usize = 2;
const STACKED_REGISTER_SIZE: usize = 4 * (RV32_GPRS_REGISTER + RV32_TOP_REGISTER);
/**
 *  Stack layout for LN_MCU_RV32  : Total = 28+2 = 30 x 32 bits register
        PC       2
        MSTATUS
        x1(ra)    28
        x5..x31

    aka
        PC
        MSTATUS
        x1/ra
        t0 t1 t2
        s0/fp
        s1
        a0--a7
        s2--a11
        t3 t4 t5 t6
*/

/**
 *
 */
struct rv32_gprs {
    sp: u32,
    pc: u32,
    mstatus: u32,
    gprs: [u32; 32], // x0...x31
    pointer: u32,
}
/**
 *
 *
 */
impl rv32_gprs {
    /**
     *
     */
    pub fn new() -> Self {
        rv32_gprs {
            sp: 0,
            pc: 0,
            mstatus: 0,
            gprs: [0; 32],
            pointer: 0,
        }
    }
    /**
     *
     */
    pub fn push(&mut self, first: usize, last: usize) -> bool {
        let to_push = last - first;
        let current = self.pointer;
        self.pointer += 4 * to_push as u32;
        bmp_write_mem32(current, &self.gprs[first..last])
    }
    /**
     *
     */
    pub fn pop(&mut self, first: usize, last: usize) -> bool {
        let current = self.pointer;
        self.pointer += 4 * (last - first) as u32;
        bmp_read_mem32(current, &mut self.gprs[first..last])
    }
    pub fn pop32(&mut self) -> u32 {
        let current = self.pointer;
        self.pointer += 4_u32;
        let mut out: [u32; 1] = [0; 1];
        bmp_read_mem32(current, &mut out[0..1]);
        out[0]
    }
    pub fn push32(&mut self, reg: u32) {
        let xin: [u32; 1] = [reg];
        bmp_write_mem32(self.pointer, &xin);
        self.pointer += 4_u32;
    }
}

/**
 *
 */
pub struct freertos_switch_handler_rv32 {
    gprs: rv32_gprs,
}
/**
 *
 */
impl freertos_switch_handler_rv32 {
    pub fn new() -> Self {
        freertos_switch_handler_rv32 {
            gprs: rv32_gprs::new(),
        }
    }
}

/**
 *
 */
impl freertos_switch_handler for freertos_switch_handler_rv32 {
    /**
     * write internal to actual registers
     */
    fn write_current_registers(&self) -> bool {
        for i in 1..31 {
            bmp_write_register(i as u32, self.gprs.gprs[i]);
        }
        // Mstatus missing
        bmp_write_register(32, self.gprs.pc);
        true
    }
    /**
     * copy actual registers to internal
     */
    fn read_current_registers(&mut self) -> bool {
        let regs = bmp_read_registers();
        if regs.len() < 33 {
            bmpwarning!("Incorrect # of registers {}", regs.len());
            return false;
        }
        self.gprs.gprs[1..32].copy_from_slice(&regs[1..32]);
        self.gprs.sp = regs[2];
        self.gprs.pc = regs[32];
        // mstatus missing
        true
    }
    /**
     * write register dump to adr, careful the register are out of order
     * We write them as if it was a freertos task switch
     */
    fn write_registers_to_stack(&mut self) -> bool {
        self.gprs.sp -= STACKED_REGISTER_SIZE as u32; // adjust stack to be at the beginning
        self.gprs.pointer = self.gprs.sp;

        self.gprs.push32(self.gprs.pc);
        self.gprs.push32(self.gprs.mstatus);

        self.gprs.push(1, 2); // x1
        self.gprs.push(5, 32); // x5..x31
        true
    }
    /**
     * read register dump from adr, careful the register are out of order
     */
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        self.gprs.sp = address + (STACKED_REGISTER_SIZE as u32);
        self.gprs.pointer = address;
        self.gprs.pc = self.gprs.pop32(); // PC
        self.gprs.mstatus = self.gprs.pop32(); // mstatus
        self.gprs.gprs[1] = self.gprs.pop32(); // ra//x1
        self.gprs.pop(5, 32); // t0 1 t2
        true
    }

    fn get_sp(&self) -> u32 {
        self.gprs.sp
    }
}
// EOF
