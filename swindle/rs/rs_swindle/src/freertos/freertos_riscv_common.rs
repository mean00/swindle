/// \brief Shared GPRs structure and OS layout detection for RISC-V FreeRTOS support
///
/// This module provides the `rv32_gprs` struct used to cache the general purpose
/// registers (`x0` through `x31`), the PC, the `mstatus` CSR, and the stack pointer 
/// offset (`pointer`) during context switching.
///
/// It also provides the handler factory `create_rv32_switch_handler()` that 
/// instantiates the correct handler layout (Standard vs. WCH/Bumblebee) based on 
/// debug symbols.

use crate::bmp::{bmp_read_mem32, bmp_write_mem32};
use crate::freertos::freertos_trait::freertos_switch_handler;
use crate::freertos::freertos_symbols::{get_symbols, LAYOUT_CH32, LAYOUT_RV_STD, LAYOUT_RV_STD_FPU};

use super::freertos_riscv_rv32_std::FreeRTOSSwitchHandlerRV32Std;
use super::freertos_riscv_rv32_wch::FreeRTOSSwitchHandlerCH32;

/// Retrieves the current RISC-V FreeRTOS stack layout type detected from debug symbols.
pub fn get_current_layout() -> u32 {
    get_symbols()
        .debug_offsets
        .map_or(LAYOUT_CH32, |offsets| offsets.layout_type)
}

pub const RV32_GPRS_REGISTER: usize = 28;
pub const RV32_TOP_REGISTER: usize = 2;
pub const STACKED_REGISTER_SIZE: usize = 4 * (RV32_GPRS_REGISTER + RV32_TOP_REGISTER);

/// Represents the internal state of a RISC-V thread during a FreeRTOS context switch.
/// Contains the stack pointer (`sp`), program counter (`pc`), Machine Status (`mstatus`),
/// and the 32 General Purpose Registers (`x0`..`x31`).
pub struct rv32_gprs {
    pub sp: u32,
    pub pc: u32,
    pub mstatus: u32,
    pub gprs: [u32; 32], // x0...x31
    pub pointer: u32,
}

impl rv32_gprs {
    /// Creates a new zeroed `rv32_gprs` instance.
    pub fn new() -> Self {
        rv32_gprs {
            sp: 0,
            pc: 0,
            mstatus: 0,
            gprs: [0; 32],
            pointer: 0,
        }
    }
    
    /// Pushes a slice of GPRs to target memory (stack) and increments the internal pointer.
    pub fn push(&mut self, first: usize, last: usize) -> bool {
        let to_push = last - first;
        let current = self.pointer;
        self.pointer += 4 * to_push as u32;
        bmp_write_mem32(current, &self.gprs[first..last])
    }
    
    /// Pops a slice of GPRs from target memory (stack) and increments the internal pointer.
    pub fn pop(&mut self, first: usize, last: usize) -> bool {
        let current = self.pointer;
        self.pointer += 4 * (last - first) as u32;
        bmp_read_mem32(current, &mut self.gprs[first..last])
    }
    
    /// Pops a single 32-bit word from target memory and increments the internal pointer.
    pub fn pop32(&mut self) -> u32 {
        let current = self.pointer;
        self.pointer += 4_u32;
        let mut out: [u32; 1] = [0; 1];
        bmp_read_mem32(current, &mut out[0..1]);
        out[0]
    }
    
    /// Pushes a single 32-bit word to target memory and increments the internal pointer.
    pub fn push32(&mut self, reg: u32) {
        let xin: [u32; 1] = [reg];
        bmp_write_mem32(self.pointer, &xin);
        self.pointer += 4_u32;
    }
}

/// Factory function that inspects the binary's debug symbols to determine which RISC-V 
/// FreeRTOS stack layout to use, and returns the appropriate heap-allocated switch handler.
pub fn create_rv32_switch_handler() -> alloc::boxed::Box<dyn freertos_switch_handler> {
    match get_current_layout() {
        LAYOUT_RV_STD | LAYOUT_RV_STD_FPU => alloc::boxed::Box::new(FreeRTOSSwitchHandlerRV32Std::new()),
        _ => alloc::boxed::Box::new(FreeRTOSSwitchHandlerCH32::new()),
    }
}
