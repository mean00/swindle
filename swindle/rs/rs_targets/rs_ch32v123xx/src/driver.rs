#![allow(dead_code)]

use super::flashstub::*;
use super::mon_commands;
use super::registers::*;
use rsbmp::commands::CommandTree;
use rsbmp::flash::{FlashOperation, StubCaps, StubState};
use rsbmp::riscv::{HwTriggerSupport, HwTriggers, RiscvHart};
use rsbmp::target::{FlashDesc, MemoryMap, PartDescriptor, RustRiscvTargetDriver};

/// Ranges shorter than this are cheaper to CRC in software than to hand to a stub (one
/// stub round-trip costs more than reading a couple of hundred bytes over the wire).
const CRC_STUB_MIN_LEN: u32 = 256;

/// How often the chip identification word is re-read during probe.
const IDENTIFY_RETRIES: usize = 8;

/// Part families this driver knows how to drive.
fn is_known_family(family: u16) -> bool {
    matches!(
        family,
        FAMILY_CH32V103
            | FAMILY_CH32V203
            | FAMILY_CH32V205
            | FAMILY_CH32V208
            | FAMILY_CH32V303
            | FAMILY_CH32V305
            | FAMILY_CH32V307
    )
}

// The CRC stub is parked between the write stub and the stub data area.
const _: () = assert!(STUB_CODE_LOCATION_CRC + CH32V3X_CRC32_BIN_LEN as u32 <= STUB_DATA_LOCATION);

/// Which flash (FPEC) generation a part has.
///
/// Everything else in this driver - chip id, memory map, RCC, CRC unit, the FPEC base and
/// its unlock keys - is the same across the CH32V1xx/V2xx/V3xx family, but not the
/// programming core:
///
/// * [`FlashGen::Chapter32`] - the FPEC of `ch32v3.md` chapter 32 (CH32V103/203/208/30x):
///   fast page program by writing the flash window then `PGSTRT`, 256 B fast page erase,
///   `BER32`/`BER64` block erase.
/// * [`FlashGen::Buffered`] - the controller of the CH32V205 (RM V1.2, chapter 27): `FTPG`,
///   `BUFRST`, `BUFLOAD`, `STRT` page-buffer programming (there is no plain `PG` mode), 2 KiB
///   sector erase (`PER`), 256 B page erase (`FTER`), 64 KiB block erase (`BER64`, bit 23).
///   Its erase and program sequences run entirely in the in-SRAM stubs: the part's debug
///   module cannot reliably read the FPEC status registers or write multi-word blocks, so no
///   flash command is driven from the probe there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashGen {
    Chapter32,
    Buffered,
}

/// Target chip metadata discovered during probe.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v123xxInfo {
    pub chipid: u32,
    pub family: u16,
    pub flash_size_kb: u32,
    pub ram_size_kb: u32,
    pub driver_name: &'static str,
    /// Flash controller generation of this part (see [`FlashGen`]).
    pub flash_gen: FlashGen,
}

/// Idiomatic pure Rust driver for WCH CH32V1xx / CH32V2xx / CH32V3xx microcontrollers.
///
/// Only what the probe *found*: the memory map is computed from these sizes on demand
/// ([`RustRiscvTargetDriver::memory_map`]), never stored here.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v123xxDriver {
    pub info: Ch32v123xxInfo,
    /// Hardware breakpoint capability, resolved at probe time by [`HwTriggers`].
    pub triggers: HwTriggerSupport,
    /// Whether the erase/write flashstubs are currently parked in SRAM.
    stubs: StubState,
}

impl Ch32v123xxDriver {
    // -------------------------------------------------------------------------------
    // Probe and target metadata
    // -------------------------------------------------------------------------------

    /// What this part is known to offer, before asking the hart.
    ///
    /// The V3xx cores (QingKe V4) are known to have execute triggers; for the
    /// V1xx/V2xx parts nothing is known for sure, so they are asked.
    pub fn hw_triggers(family: u16) -> HwTriggers {
        match family {
            FAMILY_CH32V303 | FAMILY_CH32V305 | FAMILY_CH32V307 => HwTriggers::Present,
            _ => HwTriggers::Discover,
        }
    }

    /// Probe target core and detect CH32V1xx / V2xx / V3xx chip variants and memory layouts.
    pub fn probe(hart: &mut RiscvHart) -> Option<Self> {
        // The identification word is read a few times: one failed read (the part may still be
        // resettling, and this access path is not always reliable) would otherwise silently
        // demote the part to the generic RISC-V driver, which has no flash at all.
        let mut chipid = 0u32;
        let mut family = 0u16;
        for _ in 0..IDENTIFY_RETRIES {
            if let Ok(value) = hart.mem_read32(CHIPID_ADDRESS) {
                let candidate = ((value & CHIPID_FAMILY_MASK) >> CHIPID_FAMILY_OFFSET) as u16;
                if is_known_family(candidate) {
                    chipid = value;
                    family = candidate;
                    break;
                }
            }
        }
        if !is_known_family(family) {
            return None;
        }

        let (driver_name, detect_size) = match family {
            FAMILY_CH32V103 => ("CH32V103\0", false),
            FAMILY_CH32V203 => ("CH32V203\0", false),
            FAMILY_CH32V205 => ("CH32V205\0", false),
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
                _ => match family {
                    FAMILY_CH32V205 => (CH32V205_FLASH_SIZE_KB, CH32V205_RAM_SIZE_KB),
                    _ => (64, 20),
                },
            }
        };

        // The CH32V205 is the one part of this family whose flash controller is not the one
        // `ch32v3.md` chapter 32 describes (see `FlashGen`).
        let flash_gen = if family == FAMILY_CH32V205 {
            FlashGen::Buffered
        } else {
            FlashGen::Chapter32
        };

        Some(Self {
            info: Ch32v123xxInfo {
                chipid,
                family,
                flash_size_kb,
                ram_size_kb,
                driver_name,
                flash_gen,
            },
            triggers: Self::hw_triggers(family).resolve(hart),
            // Both generations advertise erase+write: the buffered one erases from the probe
            // and parks nothing for it, but the state machine re-uploads whenever the current
            // operation is not covered, so claiming ERASE keeps one upload per session.
            stubs: StubState::new(StubCaps::ERASE.union(StubCaps::WRITE)),
        })
    }
    // -------------------------------------------------------------------------------
    // Shared helpers: register access, waits and image padding (valid for both parts)
    // -------------------------------------------------------------------------------

    #[inline(always)]
    fn read_flash_reg(&self, hart: &mut RiscvHart, addr: u32) -> u32 {
        hart.mem_read32(addr).unwrap_or(0)
    }

    #[inline(always)]
    fn write_flash_reg(&self, hart: &mut RiscvHart, addr: u32, val: u32) {
        let _ = hart.mem_write32(addr, val);
    }

    #[inline(always)]
    fn read_rcc_reg(&self, hart: &mut RiscvHart, addr: u32) -> u32 {
        hart.mem_read32(addr).unwrap_or(0)
    }

    #[inline(always)]
    fn write_rcc_reg(&self, hart: &mut RiscvHart, addr: u32, val: u32) {
        let _ = hart.mem_write32(addr, val);
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
        let mut ctl = self.read_rcc_reg(hart, rcc_reg!(ctlr));
        ctl |= RCC_CTLR_HSION;
        self.write_rcc_reg(hart, rcc_reg!(ctlr), ctl);
        let _ = self.wait_bit(hart, rcc_reg!(ctlr), RCC_CTLR_HSIRDY, 5000);

        let mut cfg0 = self.read_rcc_reg(hart, rcc_reg!(cfgr0));
        cfg0 &= !RCC_CFGR0_SW_MASK;
        cfg0 |= RCC_CFGR0_SW_HSI;
        self.write_rcc_reg(hart, rcc_reg!(cfgr0), cfg0);

        let _ = self.wait_clear(hart, rcc_reg!(cfgr0), RCC_CFGR0_SWS_MASK, 5000);

        ctl = self.read_rcc_reg(hart, rcc_reg!(ctlr));
        ctl &= !RCC_CTLR_PLLON;
        self.write_rcc_reg(hart, rcc_reg!(ctlr), ctl);

        ctl = self.read_rcc_reg(hart, rcc_reg!(ctlr));
        ctl &= !RCC_CTLR_HSEON;
        self.write_rcc_reg(hart, rcc_reg!(ctlr), ctl);
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

        self.write_rcc_reg(hart, rcc_reg!(cfgr0), cfg0);

        let mut ctl = self.read_rcc_reg(hart, rcc_reg!(ctlr));
        ctl |= RCC_CTLR_PLLON;
        self.write_rcc_reg(hart, rcc_reg!(ctlr), ctl);

        if !self.wait_bit(hart, rcc_reg!(ctlr), RCC_CTLR_PLLRDY, 5000) {
            return;
        }

        let mut cfg0_sw = self.read_rcc_reg(hart, rcc_reg!(cfgr0));
        cfg0_sw &= !RCC_CFGR0_SW_MASK;
        cfg0_sw |= RCC_CFGR0_SW_PLL;
        self.write_rcc_reg(hart, rcc_reg!(cfgr0), cfg0_sw);

        let _ = self.wait_bit(hart, rcc_reg!(cfgr0), RCC_CFGR0_SWS_PLL, 5000);
    }

    /// Set flash memory access timing (wait states / clock division).
    pub fn set_wait_state(&self, hart: &mut RiscvHart, clock_mhz: u32) {
        let mut ctl = self.read_flash_reg(hart, flash_reg!(ctlr));

        if clock_mhz <= 60 {
            ctl |= FMC_CTL_SCKMOD;
        } else {
            ctl &= !FMC_CTL_SCKMOD;
        }

        if (ctl & FMC_CTL_EHMOD) != 0 {
            ctl &= !FMC_CTL_EHMOD;
            self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
            ctl |= FMC_CTL_RSENACT;
            self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
        } else {
            self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
        }
    }

    /// Poll until STATR BUSY bit clears.
    pub fn wait_not_busy(&self, hart: &mut RiscvHart) -> bool {
        for _ in 0..FLASH_TIMEOUT_ITERS {
            let stat = self.read_flash_reg(hart, flash_reg!(statr));
            if (stat & FMC_STAT_BUSY) == 0 {
                return true;
            }
        }
        false
    }

    /// Upload one stub image into its SRAM slot.
    fn upload_stub(&self, hart: &mut RiscvHart, addr: u32, image: &[u8]) -> bool {
        match self.info.flash_gen {
            FlashGen::Chapter32 => Self::upload_stub_chapter32(hart, addr, image),
            FlashGen::Buffered => Self::upload_stub_buffered(hart, addr, image),
        }
    }

    /// Chapter 32: one bulk memory write. Its sub-word tail write works, so the image goes out
    /// exactly as it is.
    fn upload_stub_chapter32(hart: &mut RiscvHart, addr: u32, image: &[u8]) -> bool {
        hart.mem_write(addr, image).is_ok()
    }

    /// Buffered: one write per word; the trailing 1-3 bytes are zero-padded into a single
    /// extra word by [`Self::write_words_buffered`].
    fn upload_stub_buffered(hart: &mut RiscvHart, addr: u32, image: &[u8]) -> bool {
        Self::write_words_buffered(hart, addr, image)
    }

    // -------------------------------------------------------------------------------
    // Flash API: one dispatcher per entry point, dispatching on `flash_gen`
    // -------------------------------------------------------------------------------

    /// CRC32 of `[addr, addr + len)` computed by the CH32 CRC unit, driven from the
    /// in-SRAM `ch32v3x_crc32` stub.
    ///
    /// The stub takes a word-aligned start and a word count and leaves the result in `a2`,
    /// so any other range falls back to the caller's software CRC (`None`). The stub is
    /// uploaded on demand -- `qCRC` can arrive without a preceding flash session -- and the
    /// bytes it overwrites are put back afterwards, since 0x2000_1400 belongs to the target.
    pub fn crc32_stub(&mut self, hart: &mut RiscvHart, addr: u32, len: u32) -> Option<u32> {
        match self.info.flash_gen {
            FlashGen::Chapter32 => self.crc32_stub_chapter32(hart, addr, len),
            // The buffered generation has no CRC stub: its debug module corrupts the multi-word
            // copies that stub saves and restores its SRAM window with, so the caller's software
            // CRC is the only trustworthy answer there.
            FlashGen::Buffered => None,
        }
    }

    /// Prepare the stubs (and, on chapter 32 parts, the FPEC) for the next flash operation.
    ///
    /// The images are uploaded once per session, as [`StubState::needs_upload`] decides: the
    /// manager asks for a `prepare` on every operation change, and a load is an erase followed
    /// by thousands of write packets.
    pub fn prepare_flash(&mut self, hart: &mut RiscvHart, op: FlashOperation) -> bool {
        match self.info.flash_gen {
            FlashGen::Chapter32 => self.prepare_flash_chapter32(hart, op),
            FlashGen::Buffered => self.prepare_flash_buffered(hart, op),
        }
    }

    /// Erase `len` bytes at `dest`, mapping the advertised erase block size onto the
    /// controller's erase modes.
    pub fn erase_flash(&mut self, hart: &mut RiscvHart, dest: u32, len: usize) -> bool {
        match self.info.flash_gen {
            FlashGen::Chapter32 => self.erase_flash_chapter32(hart, dest, len),
            FlashGen::Buffered => self.erase_flash_buffered(hart, dest, len),
        }
    }

    /// Program data into the flash window.
    pub fn write_flash(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        match self.info.flash_gen {
            FlashGen::Chapter32 => self.write_flash_chapter32(hart, dest, data),
            FlashGen::Buffered => self.write_flash_buffered(hart, dest, data),
        }
    }

    /// Software breakpoint helper: erase the 256-byte page holding `addr`.
    pub fn sw_page_erase(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        match self.info.flash_gen {
            FlashGen::Chapter32 => self.sw_page_erase_chapter32(hart, addr),
            FlashGen::Buffered => self.sw_page_erase_buffered(hart, addr),
        }
    }

    /// Software breakpoint helper: program one 256-byte page.
    pub fn sw_page_write(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        if data.len() != FLASH_PAGE_SIZE {
            return false;
        }
        match self.info.flash_gen {
            FlashGen::Chapter32 => self.sw_page_write_chapter32(hart, addr, data),
            FlashGen::Buffered => self.sw_page_write_buffered(hart, addr, data),
        }
    }

    /// Erase geometry advertised to the flash manager: `(blocksize, erasesizes)`.
    fn erase_geometry(&self) -> (usize, &'static [usize]) {
        match self.info.flash_gen {
            FlashGen::Chapter32 => Self::erase_geometry_chapter32(),
            FlashGen::Buffered => Self::erase_geometry_buffered(),
        }
    }

    // -------------------------------------------------------------------------------
    // Chapter 32 FPEC (ch32v3.md, chapter 32): CH32V103/203/208/303/305/307
    // -------------------------------------------------------------------------------

    /// Unlock standard and fast mode flash locks.
    pub fn fast_unlock_chapter32(&self, hart: &mut RiscvHart) -> bool {
        let ctl = self.read_flash_reg(hart, flash_reg!(ctlr));
        if (ctl & FMC_CTL_LK) != 0 {
            self.write_flash_reg(hart, flash_reg!(keyr), FLASH_KEY1);
            self.write_flash_reg(hart, flash_reg!(keyr), FLASH_KEY2);
        }

        let ctl2 = self.read_flash_reg(hart, flash_reg!(ctlr));
        if (ctl2 & FMC_CTL_FASTUNLOCK) != 0 {
            self.write_flash_reg(hart, flash_reg!(modekeyr), FLASH_KEY1);
            self.write_flash_reg(hart, flash_reg!(modekeyr), FLASH_KEY2);
        }

        let v = self.read_flash_reg(hart, flash_reg!(ctlr));
        (v & FMC_CTL_FASTUNLOCK) == 0
    }

    /// Poll until STATR WR_BUSY bit clears.
    pub fn wait_not_wr_busy_chapter32(&self, hart: &mut RiscvHart) -> bool {
        for _ in 0..FLASH_TIMEOUT_ITERS {
            let stat = self.read_flash_reg(hart, flash_reg!(statr));
            if (stat & FMC_STAT_WR_BUSY) == 0 {
                return true;
            }
        }
        false
    }

    fn crc32_stub_chapter32(&mut self, hart: &mut RiscvHart, addr: u32, len: u32) -> Option<u32> {
        if len < CRC_STUB_MIN_LEN || (len & 3) != 0 || (addr & 3) != 0 {
            return None;
        }
        let end = addr.checked_add(len)?;
        if addr < STUB_WORKSPACE_END && end > STUB_WORKSPACE_START {
            return None;
        }

        let mut saved = [0u8; CH32V3X_CRC32_BIN_LEN];
        hart.mem_read(STUB_CODE_LOCATION_CRC, &mut saved).ok()?;
        if hart
            .mem_write(STUB_CODE_LOCATION_CRC, CH32V3X_CRC32_BIN)
            .is_err()
        {
            let _ = hart.mem_write(STUB_CODE_LOCATION_CRC, &saved);
            return None;
        }

        // ch32_crc(addr, len_in_words, unused): the result comes back in a2.
        let crc = hart.run_stub_with_stack_ret(
            STUB_CODE_LOCATION_CRC,
            STUB_STACKEND_LOCATION,
            addr,
            len / 4,
            0,
            0,
        );
        let _ = hart.mem_write(STUB_CODE_LOCATION_CRC, &saved);
        crc
    }

    /// Chapter 32: upload both stubs, then leave the FPEC unlocked for the session.
    fn prepare_flash_chapter32(&mut self, hart: &mut RiscvHart, op: FlashOperation) -> bool {
        if self.stubs.needs_upload(op) {
            // A stub that never reached SRAM would run as garbage later, so fail the session
            // now. Nothing is marked uploaded, so the next prepare tries again.
            if !self.upload_stub(hart, STUB_CODE_LOCATION_WRITE, CH32V3X_WRITE_BIN) {
                return false;
            }
            if !self.upload_stub(hart, STUB_CODE_LOCATION_ERASE, CH32V3X_ERASE_BIN) {
                return false;
            }
            self.stubs.uploaded();
        }
        self.fast_unlock_chapter32(hart);
        true
    }

    /// Chapter 32: 32K/64K are one BER32/BER64 block erase each, anything smaller goes to the
    /// in-SRAM stub, which iterates 256 B fast page (FTER) erases.
    fn erase_flash_chapter32(&mut self, hart: &mut RiscvHart, dest: u32, mut len: usize) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;

        while len > 0 {
            let chunk: usize;
            if len >= ERASE_BLOCK_SIZE_64K {
                chunk = ERASE_BLOCK_SIZE_64K;
                if !self.block_erase_chapter32(hart, addr, FMC_CTL_BER64) {
                    return false;
                }
            } else if len >= ERASE_BLOCK_SIZE_32K {
                chunk = ERASE_BLOCK_SIZE_32K;
                if !self.block_erase_chapter32(hart, addr, FMC_CTL_BER32) {
                    return false;
                }
            } else {
                // Below one BER32 block: let the stub erase page by page.
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
                    // The core may be left mid-image: forget the upload.
                    self.stubs.invalidate();
                    return false;
                }
            }
            addr += chunk as u32;
            len -= chunk;
        }
        true
    }

    /// Erase one hardware block starting at `addr` with the block-erase mode `mode`
    /// (FMC_CTL_BER32 or FMC_CTL_BER64), per RM 32.5.7 steps 4-9.
    fn block_erase_chapter32(&self, hart: &mut RiscvHart, addr: u32, mode: u32) -> bool {
        self.fast_unlock_chapter32(hart);
        let ctl = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl | mode);
        self.write_flash_reg(hart, flash_reg!(addr), addr);
        let ctl2 = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl2 | FMC_CTL_START);
        if !self.wait_not_busy(hart) {
            return false;
        }
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
        true
    }

    /// Program a few bytes (< 256 B) using standard half-word PG mode.
    pub fn write_few_bytes_chapter32(&self, hart: &mut RiscvHart, dest: u32, src: &[u8]) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;
        self.fast_unlock_chapter32(hart);
        let ctl = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl | FMC_CTL_PG);

        for chunk in src.chunks(2) {
            let val16 = if chunk.len() == 2 {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_le_bytes([chunk[0], 0xE5])
            };
            self.wait_not_busy(hart);
            if hart.mem_write16(addr, val16).is_err() {
                self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
                return false;
            }
            addr += 2;
            self.wait_not_busy(hart);
        }
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
        true
    }

    /// Chapter 32: in-SRAM fast-page stub for whole chunks, half-word PG mode for the small
    /// ones (`vFlashWrite` tails, a single word set by hand).
    fn write_flash_chapter32(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;
        let mut offset = 0;
        let mut remaining = data.len();

        while remaining > 0 {
            let chunk = remaining.min(ATOMIC_FLASHSTUB_WRITE);
            let slice = &data[offset..offset + chunk];

            if chunk < 256 {
                if !self.write_few_bytes_chapter32(hart, addr, slice) {
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
                    // The core may be left mid-image: forget the upload.
                    self.stubs.invalidate();
                    return false;
                }
            }
            addr += chunk as u32;
            offset += chunk;
            remaining -= chunk;
        }
        true
    }

    /// Chapter 32: 256 B fast page erase (FTER) straight from the probe.
    fn sw_page_erase_chapter32(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        let page_addr = (addr | FLASH_PHYSICAL_ADDRESS) & !0xFF;
        self.fast_unlock_chapter32(hart);
        let ctl = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl | FMC_CTL_FASTERASE);
        self.write_flash_reg(hart, flash_reg!(addr), page_addr);
        let ctl2 = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl2 | FMC_CTL_START);
        let ok = self.wait_not_busy(hart);
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
        ok
    }

    /// Chapter 32: fast 64-word programming straight from the probe.
    fn sw_page_write_chapter32(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        let mut curr_addr = (addr | FLASH_PHYSICAL_ADDRESS) & !0xFF;
        self.fast_unlock_chapter32(hart);
        let ctl = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl | FMC_CTL_FASTPROGRAM);
        self.wait_not_busy(hart);

        for chunk in data.chunks_exact(4) {
            let val32 = u32::from_le_bytes(chunk.try_into().unwrap());
            let _ = hart.mem_write32(curr_addr, val32);
            curr_addr += 4;
            self.wait_not_wr_busy_chapter32(hart);
        }

        let ctl2 = self.read_flash_reg(hart, flash_reg!(ctlr));
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl2 | FMC_CTL_FASTSTART);
        self.wait_not_busy(hart);

        let mut ctl3 = self.read_flash_reg(hart, flash_reg!(ctlr));
        ctl3 &= !FMC_CTL_PG;
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl3);

        let stat = self.read_flash_reg(hart, flash_reg!(statr));
        let success = (stat & (FMC_STAT_PG_ERR | FMC_STAT_WP_ERR)) == 0;
        self.write_flash_reg(hart, flash_reg!(statr), stat);
        self.write_flash_reg(hart, flash_reg!(ctlr), ctl);
        success
    }

    /// Chapter 32 part: 4 KiB blocks, and the manager may ask for 4K/32K/64K.
    fn erase_geometry_chapter32() -> (usize, &'static [usize]) {
        (ERASE_BLOCK_SIZE_4K, &ERASE_SIZES)
    }
    // -------------------------------------------------------------------------------
    // Buffered FPEC (CH32V205/V203 RM V1.2, chapter 27): the CH32V205
    // -------------------------------------------------------------------------------

    /// Buffered: upload both stubs. Nothing is unlocked here - the two key sequences must not be
    /// interleaved with other FPEC writes (RM V1.2, 27.4.2), so each stub unlocks itself.
    fn prepare_flash_buffered(&mut self, hart: &mut RiscvHart, op: FlashOperation) -> bool {
        if self.stubs.needs_upload(op) {
            if !self.upload_stub(hart, STUB_CODE_LOCATION_WRITE, CH32V205_WRITE_BIN) {
                return false;
            }
            if !self.upload_stub(hart, STUB_CODE_LOCATION_ERASE, CH32V205_ERASE_BIN) {
                return false;
            }
            self.stubs.uploaded();
        }
        true
    }

    /// Buffered: 64K is one BER64 block erase, anything smaller one 2 KiB sector erase (PER)
    /// per sector. Both run in the erase stub, which does the status polling on the core.
    fn erase_flash_buffered(&mut self, hart: &mut RiscvHart, dest: u32, mut len: usize) -> bool {
        let mut addr = dest | FLASH_PHYSICAL_ADDRESS;

        while len > 0 {
            let chunk = if len >= V205_ERASE_BLOCK_SIZE_64K {
                if !self.erase_buffered(hart, addr, V205_ERASE_BLOCK_SIZE_64K, FMC_CTL_V205_BER64) {
                    return false;
                }
                V205_ERASE_BLOCK_SIZE_64K
            } else {
                if !self.erase_buffered(hart, addr, V205_SECTOR_SIZE, FMC_CTL_PER) {
                    return false;
                }
                V205_SECTOR_SIZE
            };
            addr += chunk as u32;
            len -= chunk;
        }
        true
    }

    /// Run the buffered-generation erase stub: `(addr, len, mode)`.
    ///
    /// All the FPEC traffic (unlock, mode, start, status polling) happens on the core: the
    /// CH32V205's debug module returns unreliable values for the FPEC state registers, so the
    /// probe cannot judge an erase itself and takes the stub's exit code instead.
    fn erase_buffered(
        &mut self,
        hart: &mut RiscvHart,
        addr: u32,
        len: usize,
        mode: u32,
    ) -> bool {
        let hold_ms = if mode == FMC_CTL_V205_BER64 { 30 } else { 10 };
        if !hart.run_stub_with_stack_hold(
            STUB_CODE_LOCATION_ERASE,
            STUB_STACKEND_LOCATION,
            addr,
            len as u32,
            mode,
            0,
            None,
            hold_ms,
        ) {
            // The core may be left mid-operation: forget the uploads.
            self.stubs.invalidate();
            return false;
        }
        true
    }

    /// Program one 256-byte page through the buffered-generation write stub.
    fn write_page_buffered(
        &mut self,
        hart: &mut RiscvHart,
        page_addr: u32,
        page: &[u8],
    ) -> bool {
        if !Self::write_words_buffered(hart, STUB_DATA_LOCATION, page) {
            return false;
        }
        if !hart.run_stub_with_stack_hold(
            STUB_CODE_LOCATION_WRITE,
            STUB_STACKEND_LOCATION,
            page_addr,
            STUB_DATA_LOCATION,
            V205_PAGE_SIZE as u32,
            0,
            None,
            2,
        ) {
            self.stubs.invalidate();
            return false;
        }
        true
    }

    /// Buffered: program `data` one 256-byte page at a time.
    ///
    /// The controller has no plain half-word programming mode (RM V1.2, 27.3.5: bit 0 is
    /// reserved), so a write that does not cover whole pages - GDB's last packet, a small
    /// `set` - is merged with the page it lands in: read that page back, patch it, program it.
    fn write_flash_buffered(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        let mut page = [0u8; V205_PAGE_SIZE];
        let mut offset = 0usize;

        while offset < data.len() {
            let addr = (dest | FLASH_PHYSICAL_ADDRESS) + offset as u32;
            let page_base = addr & !(V205_PAGE_SIZE as u32 - 1);
            let in_page = (addr - page_base) as usize;
            let take = (V205_PAGE_SIZE - in_page).min(data.len() - offset);

            if in_page != 0 || take != V205_PAGE_SIZE {
                // Partial page: start from what the flash already holds.
                if !Self::read_words_buffered(hart, page_base, &mut page) {
                    return false;
                }
            }
            page[in_page..in_page + take].copy_from_slice(&data[offset..offset + take]);

            if !self.write_page_buffered(hart, page_base, &page) {
                return false;
            }
            offset += take;
        }
        true
    }

    /// Buffered: one 256 B page (FTER, bit 17), erased by the stub.
    fn sw_page_erase_buffered(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        let page_addr = (addr | FLASH_PHYSICAL_ADDRESS) & !0xFF;
        self.prepare_flash_buffered(hart, FlashOperation::Erase)
            && self.erase_buffered(hart, page_addr, V205_PAGE_SIZE, FMC_CTL_V205_FTER)
    }

    /// Buffered: the page-buffer stub does the programming.
    fn sw_page_write_buffered(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        let page_addr = (addr | FLASH_PHYSICAL_ADDRESS) & !0xFF;
        self.prepare_flash_buffered(hart, FlashOperation::Write)
            && self.write_page_buffered(hart, page_addr, data)
    }

    /// Write `bytes` to target RAM one 32-bit word at a time.
    ///
    /// The QingKe V3B debug module of the CH32V205 corrupts multi-word abstract writes -- its
    /// own memory benchmark reports `Abstract: FAILED`, and a bulk upload lands scrambled data
    /// while single-word writes are exact. Images and page buffers therefore take this path on
    /// that part; the chapter 32 parts keep the (faster) bulk write.
    fn write_words_buffered(hart: &mut RiscvHart, addr: u32, bytes: &[u8]) -> bool {
        let mut index = 0usize;
        while index < bytes.len() {
            let mut word = [0u8; 4];
            let take = (bytes.len() - index).min(4);
            word[..take].copy_from_slice(&bytes[index..index + take]);
            if hart
                .mem_write32(addr + index as u32, u32::from_le_bytes(word))
                .is_err()
            {
                return false;
            }
            index += 4;
        }
        true
    }

    /// Read `bytes` from target RAM one 32-bit word at a time (the multi-word abstract read
    /// is unreliable on this part, see [`Self::write_words`]).
    fn read_words_buffered(hart: &mut RiscvHart, addr: u32, bytes: &mut [u8]) -> bool {
        let mut index = 0usize;
        while index < bytes.len() {
            let Ok(value) = hart.mem_read32(addr + index as u32) else {
                return false;
            };
            let raw = value.to_le_bytes();
            let take = (bytes.len() - index).min(4);
            bytes[index..index + take].copy_from_slice(&raw[..take]);
            index += 4;
        }
        true
    }

    /// Buffered part: 2 KiB sectors, and the manager may ask for a 2K sector or a 64K block.
    fn erase_geometry_buffered() -> (usize, &'static [usize]) {
        (V205_SECTOR_SIZE, &V205_ERASE_SIZES)
    }
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () =
    assert!(core::mem::size_of::<Ch32v123xxDriver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for Ch32v123xxDriver {
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
        self.triggers.has_exec
    }

    fn memory_map(&self) -> MemoryMap {
        let ram_bytes = (self.info.ram_size_kb as usize) * 1024;
        let flash_bytes = (self.info.flash_size_kb as usize) * 1024;

        let (blocksize, erasesizes) = self.erase_geometry();

        let mut map = MemoryMap::new().with_ram(RAM_ADDRESS, ram_bytes);

        // The part answers at both the physical flash base and its cached alias, so the
        // debugger may load through either window.
        for start in [FLASH_ALIAS_ADDRESS, FLASH_PHYSICAL_ADDRESS] {
            map = map.with_flash_desc(FlashDesc {
                start,
                length: flash_bytes,
                blocksize,
                writesize: FLASH_WRITE_SIZE,
                erasesizes,
                erased: 0xff,
            });
        }

        map
    }

    fn commands(&self) -> &'static [CommandTree] {
        &mon_commands::COMMANDS
    }
}

impl RustRiscvTargetDriver for Ch32v123xxDriver {
    fn hw_breakpoint_count(&self) -> u32 {
        self.triggers.exec_slots as u32
    }

    fn flash_prepare(&mut self, hart: &mut RiscvHart, op: FlashOperation) -> bool {
        // No engine to keep: this family's controller keeps no save-area of its own (the
        // probe results are all the driver holds), the session state is the stub in SRAM.
        self.prepare_flash(hart, op)
    }

    fn flash_done(&mut self, _hart: &mut RiscvHart) -> bool {
        // The controller stays unlocked and the session handed the target back: the RAM the
        // stubs live in belongs to the application again.
        self.stubs.invalidate();
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

    fn crc32(&mut self, hart: &mut RiscvHart, addr: u32, len: u32) -> Option<u32> {
        self.crc32_stub(hart, addr, len)
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
