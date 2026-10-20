#![allow(dead_code)]

use super::registers::*;
use rsbmp::riscv::RiscvHart;

/// Target device metadata discovered during probe.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v00xInfo {
    pub idcode: u32,
    pub family: u32,
    pub flash_size_kb: u32,
    pub ram_size_kb: u32,
    pub page_size: u32,
    pub driver_name: &'static str,
}

/// Idiomatic Rust driver for WCH CH32V00x / CH32V006 RISC-V microcontrollers.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v00xDriver {
    pub info: Ch32v00xInfo,
}

impl Ch32v00xDriver {
    /// Probe target core and detect chip variant and memory sizes.
    pub fn probe(hart: &mut RiscvHart) -> Option<Self> {
        let idcode = hart.mem_read32(CH32V003X_IDCODE).ok()?;
        let masked = idcode & CH32V0X_IDCODE_MASK;

        let flash_cap = hart.mem_read16(CH32V0X_ESIG_FLASH_CAP).unwrap_or(0) as u32;

        let (driver_name, ram_size_kb, page_size, family, flash_size_kb) = if masked == IDCODE_CH32V006 {
            // If OTP/IDCODE is all 1s and flash size is 62 KB, it is a CH32V006
            if flash_cap == 62 {
                ("CH32V006\0", 8, 256, 6, 62)
            } else {
                return None;
            }
        } else {
            match masked {
                IDCODE_CH32V003_F4P6
                | IDCODE_CH32V003_F4U6
                | IDCODE_CH32V003_A4M6
                | IDCODE_CH32V003_J4M6 => {
                    let flash_sz = if flash_cap != 0 { flash_cap } else { 16 };
                    ("CH32V003\0", 2, 64, 3, flash_sz)
                }
                _ => return None,
            }
        };

        Some(Self {
            info: Ch32v00xInfo {
                idcode,
                family,
                flash_size_kb,
                ram_size_kb,
                page_size,
                driver_name,
            },
        })
    }

    #[inline(always)]
    fn read_reg(&self, hart: &mut RiscvHart, offset: u32) -> u32 {
        hart.mem_read32(FlashRegisters::reg_addr(offset)).unwrap_or(0)
    }

    #[inline(always)]
    fn write_reg(&self, hart: &mut RiscvHart, offset: u32, val: u32) {
        let _ = hart.mem_write32(FlashRegisters::reg_addr(offset), val);
    }

    /// Unlock FPEC controller and extended fast programming mode.
    pub fn unlock(&self, hart: &mut RiscvHart) -> bool {
        let ctlr = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);
        if (ctlr & FMC_CTL_LOCK) != 0 {
            self.write_reg(hart, FlashRegisters::KEYR_OFFSET, FLASH_KEY1);
            self.write_reg(hart, FlashRegisters::KEYR_OFFSET, FLASH_KEY2);
        }

        let ctlr2 = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);
        if (ctlr2 & FMC_CTL_FLOCK) != 0 {
            self.write_reg(hart, FlashRegisters::MODEKEYR_OFFSET, FLASH_KEY1);
            self.write_reg(hart, FlashRegisters::MODEKEYR_OFFSET, FLASH_KEY2);
        }

        let final_ctlr = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);
        (final_ctlr & (FMC_CTL_LOCK | FMC_CTL_FLOCK)) == 0
    }

    /// Wait until FPEC busy flag is cleared.
    pub fn wait_not_busy(&self, hart: &mut RiscvHart) -> bool {
        for _ in 0..FLASH_TIMEOUT_ITERS {
            let stat = self.read_reg(hart, FlashRegisters::STATR_OFFSET);
            if (stat & FMC_STAT_BUSY) == 0 {
                return true;
            }
        }
        false
    }

    /// Consume EOP flag and check for write protection errors.
    pub fn check_complete(&self, hart: &mut RiscvHart, _addr: u32) -> bool {
        let stat = self.read_reg(hart, FlashRegisters::STATR_OFFSET);
        self.write_reg(hart, FlashRegisters::STATR_OFFSET, FMC_STAT_EOP | FMC_STAT_WRPRTERR);
        (stat & FMC_STAT_WRPRTERR) == 0
    }

    /// Fast page erase (64 B on CH32V003, 256 B on CH32V006).
    pub fn erase(&self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool {
        let page_size = self.info.page_size as usize;
        if !self.unlock(hart) || !self.wait_not_busy(hart) {
            return false;
        }

        let ctlr_saved = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);
        self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTER);

        let mut success = true;
        let mut offset = 0;
        while offset < len {
            let page_addr = ((addr + offset as u32) | FLASH_PHYSICAL_ADDRESS) as u32;

            self.write_reg(hart, FlashRegisters::ADDR_OFFSET, page_addr);
            self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTER | FMC_CTL_STRT);

            if !self.wait_not_busy(hart) || !self.check_complete(hart, page_addr) {
                success = false;
                break;
            }
            offset += page_size;
        }

        self.write_reg(hart, FlashRegisters::CTLR_OFFSET, ctlr_saved);
        success
    }

    /// Direct flash programming without stub (for small writes or breakpoints).
    pub fn write_direct(&self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        let page_size = self.info.page_size as usize;
        let len = data.len();

        if (dest as usize % page_size) != 0 || (len % page_size) != 0 {
            return false;
        }

        if !self.unlock(hart) || !self.wait_not_busy(hart) {
            return false;
        }

        let ctlr_saved = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);
        let mut success = true;
        let mut offset = 0;

        while offset < len {
            let page_base = ((dest + offset as u32) | FLASH_PHYSICAL_ADDRESS) as u32;

            self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTPG);
            self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTPG | FMC_CTL_BUFRST);
            let _ = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);

            if !self.wait_not_busy(hart) || !self.check_complete(hart, page_base) {
                success = false;
                break;
            }

            let words_in_page = page_size / FLASH_WORD_BYTES;
            let mut curr_dest = page_base;
            for w in 0..words_in_page {
                let byte_idx = offset + w * FLASH_WORD_BYTES;
                let word_bytes = [
                    data[byte_idx],
                    data[byte_idx + 1],
                    data[byte_idx + 2],
                    data[byte_idx + 3],
                ];
                let val = u32::from_le_bytes(word_bytes);

                if hart.mem_write32(curr_dest, val).is_err() {
                    success = false;
                    break;
                }

                self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTPG | FMC_CTL_BUFLOAD);
                let _ = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);

                if !self.wait_not_busy(hart) {
                    success = false;
                    break;
                }
                curr_dest += 4;
            }

            if !success {
                break;
            }

            self.write_reg(hart, FlashRegisters::ADDR_OFFSET, page_base);
            self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTPG | FMC_CTL_STRT);
            let _ = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);

            if !self.wait_not_busy(hart) || !self.check_complete(hart, page_base) {
                success = false;
                break;
            }

            offset += page_size;
        }

        self.write_reg(hart, FlashRegisters::CTLR_OFFSET, ctlr_saved);
        success
    }

    /// Load the write stub into target SRAM.
    pub fn prepare_stub(&self, hart: &mut RiscvHart) -> bool {
        hart.mem_write(STUB_CODE_LOCATION, CH32V0X_WRITE_BIN).is_ok()
    }

    /// High-speed flash programming via in-SRAM flashstub.
    pub fn write_stub(&self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        if data.len() < 256 {
            return self.write_direct(hart, dest, data);
        }

        if !self.unlock(hart) || !self.wait_not_busy(hart) {
            return false;
        }

        let ctlr_saved = self.read_reg(hart, FlashRegisters::CTLR_OFFSET);
        let mut remaining = data.len();
        let mut curr_addr = (dest | FLASH_PHYSICAL_ADDRESS) as u32;
        let mut src_offset = 0;
        let page_size = self.info.page_size;

        while remaining > 0 {
            let chunk = remaining.min(STUB_MAX_CHUNK);
            let chunk_data = &data[src_offset..src_offset + chunk];

            if hart.mem_write(STUB_DATA_LOCATION, chunk_data).is_err() {
                self.write_reg(hart, FlashRegisters::CTLR_OFFSET, ctlr_saved);
                return false;
            }

            self.write_reg(hart, FlashRegisters::CTLR_OFFSET, FMC_CTL_FTPG);

            let stub_ok = hart.run_stub_with_stack(
                STUB_CODE_LOCATION,
                STUB_STACK_LOCATION,
                curr_addr,
                STUB_DATA_LOCATION,
                chunk as u32,
                page_size,
            );

            let _ = hart.halt_request();

            if !stub_ok {
                self.write_reg(hart, FlashRegisters::CTLR_OFFSET, ctlr_saved);
                return false;
            }

            curr_addr += chunk as u32;
            src_offset += chunk;
            remaining -= chunk;
        }

        self.write_reg(hart, FlashRegisters::CTLR_OFFSET, ctlr_saved);
        true
    }

    /// Software breakpoint page erase helper.
    pub fn page_erase(&self, hart: &mut RiscvHart, addr: u32) -> bool {
        let page_size = self.info.page_size;
        if (addr & (page_size - 1)) != 0 {
            return false;
        }
        self.erase(hart, addr, page_size as usize)
    }

    /// Software breakpoint page write helper.
    pub fn page_write(&self, hart: &mut RiscvHart, addr: u32, src: &[u8], page_size: u32) -> bool {
        if page_size != self.info.page_size || (addr & (page_size - 1)) != 0 {
            return false;
        }
        self.write_direct(hart, addr, src)
    }
}
