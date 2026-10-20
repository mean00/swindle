#![allow(dead_code)]

use super::registers::*;
use rsbmp::riscv::RiscvHart;
use rsbmp::target::{FlashDesc, RamDesc, RustTargetDriver};

/// Target chip metadata discovered during probe.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v123xxInfo {
    pub chipid: u32,
    pub family: u16,
    pub flash_size_kb: u32,
    pub ram_size_kb: u32,
    pub driver_name: &'static str,
}

/// Idiomatic pure Rust driver for WCH CH32V1xx / CH32V2xx / CH32V3xx microcontrollers.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v123xxDriver {
    pub info: Ch32v123xxInfo,
    pub ram_desc: [RamDesc; 1],
    pub flash_desc: [FlashDesc; 2],
}

impl Ch32v123xxDriver {
    /// Probe target core and detect CH32V1xx / V2xx / V3xx chip variants and memory layouts.
    pub fn probe(hart: &mut RiscvHart) -> Option<Self> {
        let chipid = hart.mem_read32(CHIPID_ADDRESS).ok()?;
        let family = ((chipid & CHIPID_FAMILY_MASK) >> CHIPID_FAMILY_OFFSET) as u16;

        let (driver_name, detect_size) = match family {
            FAMILY_CH32V103 => ("CH32V103\0", false),
            FAMILY_CH32V203 => ("CH32V203\0", false),
            FAMILY_CH32V208 => ("CH32V208\0", false),
            FAMILY_CH32V303 => ("CH32V303\0", true),
            FAMILY_CH32V305 => ("CH32V305\0", true),
            FAMILY_CH32V307 => ("CH32V307\0", true),
            _ => return None,
        };

        let (flash_size_kb, ram_size_kb) = if detect_size {
            let user_opt = hart.mem_read32(OPTION_BYTES_ADDRESS).unwrap_or(0);
            let layout = (user_opt >> 21) & 0x7;
            match layout {
                0 | 1 => (192, 128),
                2 | 3 => (224, 96),
                4 | 5 => (256, 64),
                6 | 7 => (228, 32),
                _ => (128, 32),
            }
        } else {
            match chipid {
                0x20310500 => (64, 20),
                0x2080053c => (480, 64),
                _ => (64, 20),
            }
        };

        let ram_bytes = (ram_size_kb as usize) * 1024;
        let flash_bytes = (flash_size_kb as usize) * 1024;

        let ram_desc = [RamDesc {
            start: RAM_ADDRESS,
            length: ram_bytes,
        }];

        let flash_desc = [
            FlashDesc {
                start: FLASH_ALIAS_ADDRESS,
                length: flash_bytes,
                blocksize: FLASH_BLOCK_SIZE,
                writesize: FLASH_WRITE_SIZE,
                erasesizes: &ERASE_SIZES,
                erased: 0xff,
            },
            FlashDesc {
                start: FLASH_PHYSICAL_ADDRESS,
                length: flash_bytes,
                blocksize: FLASH_BLOCK_SIZE,
                writesize: FLASH_WRITE_SIZE,
                erasesizes: &ERASE_SIZES,
                erased: 0xff,
            },
        ];

        Some(Self {
            info: Ch32v123xxInfo {
                chipid,
                family,
                flash_size_kb,
                ram_size_kb,
                driver_name,
            },
            ram_desc,
            flash_desc,
        })
    }

    #[inline(always)]
    fn read_flash_reg(&self, hart: &mut RiscvHart, offset: u32) -> u32 {
        hart.mem_read32(FlashRegisters::reg_addr(offset)).unwrap_or(0)
    }

    #[inline(always)]
    fn write_flash_reg(&self, hart: &mut RiscvHart, offset: u32, val: u32) {
        let _ = hart.mem_write32(FlashRegisters::reg_addr(offset), val);
    }

    #[inline(always)]
    fn read_rcc_reg(&self, hart: &mut RiscvHart, offset: u32) -> u32 {
        hart.mem_read32(rcc_reg_addr(offset)).unwrap_or(0)
    }

    #[inline(always)]
    fn write_rcc_reg(&self, hart: &mut RiscvHart, offset: u32, val: u32) {
        let _ = hart.mem_write32(rcc_reg_addr(offset), val);
    }

    fn wait_bit(&self, hart: &mut RiscvHart, addr: u32, mask: u32, iters: usize) -> bool {
        for _ in 0..iters {
            if let Ok(val) = hart.mem_read32(addr) {
                if (val & mask) != 0 {
                    return true;
                }
            }
        }
        false
    }

    fn wait_clear(&self, hart: &mut RiscvHart, addr: u32, mask: u32, iters: usize) -> bool {
        for _ in 0..iters {
            if let Ok(val) = hart.mem_read32(addr) {
                if (val & mask) == 0 {
                    return true;
                }
            }
        }
        false
    }

    /// Revert target system clock to clean 8 MHz HSI baseline.
    pub fn revert_to_internal_8m(&self, hart: &mut RiscvHart) {
        let mut ctl = self.read_rcc_reg(hart, RCC_CTLR_OFFSET);
        ctl |= RCC_CTLR_HSION;
        self.write_rcc_reg(hart, RCC_CTLR_OFFSET, ctl);
        let _ = self.wait_bit(hart, rcc_reg_addr(RCC_CTLR_OFFSET), RCC_CTLR_HSIRDY, 5000);

        let mut cfg0 = self.read_rcc_reg(hart, RCC_CFGR0_OFFSET);
        cfg0 &= !RCC_CFGR0_SW_MASK;
        cfg0 |= RCC_CFGR0_SW_HSI;
        self.write_rcc_reg(hart, RCC_CFGR0_OFFSET, cfg0);

        let _ = self.wait_clear(hart, rcc_reg_addr(RCC_CFGR0_OFFSET), RCC_CFGR0_SWS_MASK, 5000);

        ctl = self.read_rcc_reg(hart, RCC_CTLR_OFFSET);
        ctl &= !RCC_CTLR_PLLON;
        self.write_rcc_reg(hart, RCC_CTLR_OFFSET, ctl);

        ctl = self.read_rcc_reg(hart, RCC_CTLR_OFFSET);
        ctl &= !RCC_CTLR_HSEON;
        self.write_rcc_reg(hart, RCC_CTLR_OFFSET, ctl);
    }

    /// Configure target system clock for 60 MHz (HSI + PLL) for fast, safe flash operations.
    pub fn switch_to_60_hsi(&self, hart: &mut RiscvHart) {
        self.revert_to_internal_8m(hart);

        let cfg0 = RCC_CFGR0_SW_HSI
            | RCC_CFGR0_HPRE_DIV1
            | RCC_CFGR0_PPRE1_DIV1
            | RCC_CFGR0_PPRE2_DIV1
            | RCC_CFGR0_PLLSRC_HSI
            | rcc_cfg_pllmul(0x0D); // x15 = 60 MHz

        self.write_rcc_reg(hart, RCC_CFGR0_OFFSET, cfg0);

        let mut ctl = self.read_rcc_reg(hart, RCC_CTLR_OFFSET);
        ctl |= RCC_CTLR_PLLON;
        self.write_rcc_reg(hart, RCC_CTLR_OFFSET, ctl);

        if !self.wait_bit(hart, rcc_reg_addr(RCC_CTLR_OFFSET), RCC_CTLR_PLLRDY, 5000) {
            return;
        }

        let mut cfg0_sw = self.read_rcc_reg(hart, RCC_CFGR0_OFFSET);
        cfg0_sw &= !RCC_CFGR0_SW_MASK;
        cfg0_sw |= RCC_CFGR0_SW_PLL;
        self.write_rcc_reg(hart, RCC_CFGR0_OFFSET, cfg0_sw);

        let _ = self.wait_bit(hart, rcc_reg_addr(RCC_CFGR0_OFFSET), RCC_CFGR0_SWS_PLL, 5000);
    }

    /// Set flash memory access timing (wait states / clock division).
    pub fn set_wait_state(&self, hart: &mut RiscvHart, clock_mhz: u32) {
        let mut ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);

        if clock_mhz <= 60 {
            ctl |= FMC_CTL_SCKMOD;
        } else {
            ctl &= !FMC_CTL_SCKMOD;
        }

        if (ctl & FMC_CTL_EHMOD) != 0 {
            ctl &= !FMC_CTL_EHMOD;
            self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
            ctl |= FMC_CTL_RSENACT;
            self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
        } else {
            self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
        }
    }

    /// Unlock standard and fast mode flash locks.
    pub fn fast_unlock(&self, hart: &mut RiscvHart) -> bool {
        let ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        if (ctl & FMC_CTL_LK) != 0 {
            self.write_flash_reg(hart, FlashRegisters::KEYR_OFFSET, FLASH_KEY1);
            self.write_flash_reg(hart, FlashRegisters::KEYR_OFFSET, FLASH_KEY2);
        }

        let ctl2 = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        if (ctl2 & FMC_CTL_FASTUNLOCK) != 0 {
            self.write_flash_reg(hart, FlashRegisters::MODEKEYR_OFFSET, FLASH_KEY1);
            self.write_flash_reg(hart, FlashRegisters::MODEKEYR_OFFSET, FLASH_KEY2);
        }

        let v = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        (v & FMC_CTL_FASTUNLOCK) == 0
    }

    /// Poll until STATR BUSY bit clears.
    pub fn wait_not_busy(&self, hart: &mut RiscvHart) -> bool {
        for _ in 0..FLASH_TIMEOUT_ITERS {
            let stat = self.read_flash_reg(hart, FlashRegisters::STATR_OFFSET);
            if (stat & FMC_STAT_BUSY) == 0 {
                return true;
            }
        }
        false
    }

    /// Poll until STATR WR_BUSY bit clears.
    pub fn wait_not_wr_busy(&self, hart: &mut RiscvHart) -> bool {
        for _ in 0..FLASH_TIMEOUT_ITERS {
            let stat = self.read_flash_reg(hart, FlashRegisters::STATR_OFFSET);
            if (stat & FMC_STAT_WR_BUSY) == 0 {
                return true;
            }
        }
        false
    }

    /// Prepare stubs and clock for upcoming flash programming.
    pub fn prepare_flash(&mut self, hart: &mut RiscvHart) -> bool {
        let _ = hart.mem_write(STUB_CODE_LOCATION_ERASE, CH32V3X_ERASE_BIN);
        let _ = hart.mem_write(STUB_CODE_LOCATION_WRITE, CH32V3X_WRITE_BIN);
        self.fast_unlock(hart);

        // self.switch_to_60_hsi(hart);
        // self.set_wait_state(hart, 60);
        true
    }

    /// Erase flash range using hardware BER64, BER32, or in-SRAM erase stub.
    pub fn erase_flash(&mut self, hart: &mut RiscvHart, dest: u32, mut len: usize) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;

        while len > 0 {
            let chunk: usize;
            if len >= 65536 {
                chunk = 65536;
                self.fast_unlock(hart);
                let ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl | FMC_CTL_BER64);
                self.write_flash_reg(hart, FlashRegisters::ADDR_OFFSET, addr);
                let ctl2 = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl2 | FMC_CTL_START);
                if !self.wait_not_busy(hart) {
                    return false;
                }
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
            } else if len >= 32768 {
                chunk = 32768;
                self.fast_unlock(hart);
                let ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl | FMC_CTL_BER32);
                self.write_flash_reg(hart, FlashRegisters::ADDR_OFFSET, addr);
                let ctl2 = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl2 | FMC_CTL_START);
                if !self.wait_not_busy(hart) {
                    return false;
                }
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
            } else {
                chunk = len.min(ATOMIC_FLASHSTUB_WRITE);
                let stub_ok = hart.run_stub_with_stack(
                    STUB_CODE_LOCATION_ERASE,
                    STUB_STACKEND_LOCATION,
                    addr,
                    chunk as u32,
                    0,
                    0,
                );
                if !stub_ok {
                    return false;
                }
            }
            addr += chunk as u32;
            len -= chunk;
        }
        true
    }

    /// Program a few bytes (< 256 B) using standard half-word PG mode.
    pub fn write_few_bytes(&self, hart: &mut RiscvHart, dest: u32, src: &[u8]) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;
        self.fast_unlock(hart);
        let ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl | FMC_CTL_PG);

        for chunk in src.chunks(2) {
            let val16 = if chunk.len() == 2 {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_le_bytes([chunk[0], 0xE5])
            };
            self.wait_not_busy(hart);
            if hart.mem_write16(addr, val16).is_err() {
                self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
                return false;
            }
            addr += 2;
            self.wait_not_busy(hart);
        }
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
        true
    }

    /// Program data using in-SRAM flashstub or half-word mode for small writes.
    pub fn write_flash(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;
        let mut offset = 0;
        let mut remaining = data.len();

        while remaining > 0 {
            let chunk = remaining.min(ATOMIC_FLASHSTUB_WRITE);
            let slice = &data[offset..offset + chunk];

            if chunk < 256 {
                if !self.write_few_bytes(hart, addr, slice) {
                    return false;
                }
            } else {
                if hart.mem_write(STUB_DATA_LOCATION, slice).is_err() {
                    return false;
                }
                let stub_ok = hart.run_stub_with_stack(
                    STUB_CODE_LOCATION_WRITE,
                    STUB_STACKEND_LOCATION,
                    addr,
                    STUB_DATA_LOCATION,
                    chunk as u32,
                    0,
                );
                if !stub_ok {
                    return false;
                }
            }
            addr += chunk as u32;
            offset += chunk;
            remaining -= chunk;
        }
        true
    }

    /// Software breakpoint 256-byte page erase helper.
    pub fn sw_page_erase(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        let page_addr = (addr | FLASH_PHYSICAL_ADDRESS) & !0xFF;
        self.fast_unlock(hart);
        let ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl | FMC_CTL_FASTERASE);
        self.write_flash_reg(hart, FlashRegisters::ADDR_OFFSET, page_addr);
        let ctl2 = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl2 | FMC_CTL_START);
        let ok = self.wait_not_busy(hart);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
        ok
    }

    /// Software breakpoint 256-byte page write helper (fast 64-word programming).
    pub fn sw_page_write(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        if data.len() != FLASH_PAGE_SIZE {
            return false;
        }
        let mut curr_addr = (addr | FLASH_PHYSICAL_ADDRESS) & !0xFF;
        self.fast_unlock(hart);
        let ctl = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl | FMC_CTL_FASTPROGRAM);
        self.wait_not_busy(hart);

        for chunk in data.chunks_exact(4) {
            let val32 = u32::from_le_bytes(chunk.try_into().unwrap());
            let _ = hart.mem_write32(curr_addr, val32);
            curr_addr += 4;
            self.wait_not_wr_busy(hart);
        }

        let ctl2 = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl2 | FMC_CTL_FASTSTART);
        self.wait_not_busy(hart);

        let mut ctl3 = self.read_flash_reg(hart, FlashRegisters::CTLR_OFFSET);
        ctl3 &= !FMC_CTL_PG;
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl3);

        let stat = self.read_flash_reg(hart, FlashRegisters::STATR_OFFSET);
        let success = (stat & (FMC_STAT_PG_ERR | FMC_STAT_WP_ERR)) == 0;
        self.write_flash_reg(hart, FlashRegisters::STATR_OFFSET, stat);
        self.write_flash_reg(hart, FlashRegisters::CTLR_OFFSET, ctl);
        success
    }
}

impl RustTargetDriver for Ch32v123xxDriver {
    fn name(&self) -> &'static str {
        self.info.driver_name
    }

    fn part_id(&self) -> u32 {
        self.info.chipid
    }

    fn target_options(&self) -> u32 {
        0
    }

    fn has_hw_breakpoints(&self) -> bool {
        false
    }

    fn ram_regions(&self) -> &[RamDesc] {
        &self.ram_desc
    }

    fn flash_regions(&self) -> &[FlashDesc] {
        &self.flash_desc
    }

    fn flash_prepare(&mut self, hart: &mut RiscvHart) -> bool {
        self.prepare_flash(hart)
    }

    fn flash_done(&mut self, _hart: &mut RiscvHart) -> bool {
        // self.revert_to_internal_8m(hart);
        // self.set_wait_state(hart, 8);
        true
    }

    fn flash_erase(&mut self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool {
        self.erase_flash(hart, addr, len)
    }

    fn flash_write(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        self.write_flash(hart, dest, data)
    }

    fn sw_page_size(&self) -> u32 {
        FLASH_PAGE_SIZE as u32
    }

    fn sw_page_erase(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        self.sw_page_erase(hart, addr)
    }

    fn sw_page_write(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        self.sw_page_write(hart, addr, data)
    }
}
