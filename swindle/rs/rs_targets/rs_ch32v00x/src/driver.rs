#![allow(dead_code)]

use super::flashstub::*;
use super::registers::*;
use rsbmp::flash::{FlashOperation, StubCaps, StubState};
use rsbmp::riscv::{HwTriggerSupport, HwTriggers, RiscvHart};
use rsbmp::target::{FlashDesc, MemoryMap, PartDescriptor, RustRiscvTargetDriver};

/// How long the page-write stub is given to report back, in milliseconds.
///
/// The work is about a millisecond, so this is generous; it only has to keep a broken
/// stub from costing the flash session a long stall.
const STUB_RUN_TIMEOUT_MS: u32 = 500;

/// Per-page silence for the page-write stub, in ms, for this part family.
///
/// Zero - poll from the first millisecond - and that is a measurement, not an assumption.
/// The CH32V006 was thought to corrupt its own flash programming when its debug module is
/// accessed while the stub runs (the 2024 sweep in doc/ch32v00x.md, which is what bought it
/// 30 ms per page). Re-measured on the shipping code: 16 completed 2 KB loads at 0, 10, 20
/// and 30 ms per page, every block verified in a *fresh* session afterwards, all exact - and
/// with the stub's halt now armed (`dcsr.ebreakm`) it is reported on every run with ~0.5 ms
/// of polling. The reason the old number no longer applies is the poll cadence: with one
/// `dmstatus` sample per millisecond over SDI, the first sample lands *after* the stub has
/// finished, so the silence never covered a window the poll could have hit.
///
/// The value is per hardware page because the stub programs one page at a time, so the
/// silence scales with `chunk / page_size`: 2 pages for a 512-byte block on a CH32V006
/// (256-byte pages), 8 on a CH32V003 (64-byte pages). See `write_stub()`. A family that
/// turns out to need a silence gets its number here; `mon stub_hold <ms>` overrides it at
/// runtime for a sweep without a rebuild.
const fn stub_hold_per_page(_family: u32) -> u32 {
    0
}

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

/// Idiomatic pure Rust driver for WCH CH32V00x / CH32V006 microcontrollers.
///
/// Only what the probe *found*: the memory map is computed from these sizes on demand
/// ([`RustRiscvTargetDriver::memory_map`]), never stored here.
#[derive(Debug, Clone, Copy)]
pub struct Ch32v00xDriver {
    pub info: Ch32v00xInfo,
    /// Hardware breakpoint capability, resolved at probe time by [`HwTriggers`].
    pub triggers: HwTriggerSupport,
    /// Whether the programming flashstub is currently parked in SRAM.
    stubs: StubState,
}

impl Ch32v00xDriver {
    /// What this part is known to offer, before asking the hart.
    ///
    /// The CH32V003 core has no trigger unit at all, so there is nothing to ask and no
    pub fn hw_triggers(_family: u32) -> HwTriggers {
        HwTriggers::Absent
    }

    /// Probe target core and detect chip variant and memory sizes.
    pub fn probe(hart: &mut RiscvHart) -> Option<Self> {
        let idcode = hart.mem_read32(CH32V003X_IDCODE).ok()?;
        let masked = idcode & CH32V0X_IDCODE_MASK;

        let flash_cap = hart.mem_read16(CH32V0X_ESIG_FLASH_CAP).unwrap_or(0) as u32;

        let (driver_name, ram_size_kb, page_size, family, flash_size_kb) =
            if masked == IDCODE_CH32V006 {
                // If OTP/IDCODE is all 1s and flash size is 62 KB, it is a CH32V006
                if flash_cap == 62 {
                    ("CH32V006\0", 8, 256, FAMILY_CH32V006, 62)
                } else {
                    return None;
                }
            } else {
                match masked {
                    IDCODE_CH32V003_F4P6 | IDCODE_CH32V003_F4U6 | IDCODE_CH32V003_A4M6
                    | IDCODE_CH32V003_J4M6 => {
                        let flash_sz = if flash_cap != 0 { flash_cap } else { 16 };
                        ("CH32V003\0", 2, 64, FAMILY_CH32V003, flash_sz)
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
            triggers: Self::hw_triggers(family).resolve(hart),
            stubs: StubState::new(StubCaps::WRITE),
        })
    }

    #[inline(always)]
    fn read_reg(&self, hart: &mut RiscvHart, addr: u32) -> u32 {
        hart.mem_read32(addr).unwrap_or(0)
    }

    #[inline(always)]
    fn write_reg(&self, hart: &mut RiscvHart, addr: u32, val: u32) {
        let _ = hart.mem_write32(addr, val);
    }

    /// Unlock FPEC controller and extended fast programming mode.
    pub fn unlock(&self, hart: &mut RiscvHart) -> bool {
        let ctlr = self.read_reg(hart, flash_reg!(ctlr));
        if (ctlr & FMC_CTL_LOCK) != 0 {
            self.write_reg(hart, flash_reg!(keyr), FLASH_KEY1);
            self.write_reg(hart, flash_reg!(keyr), FLASH_KEY2);
        }

        let ctlr2 = self.read_reg(hart, flash_reg!(ctlr));
        if (ctlr2 & FMC_CTL_FLOCK) != 0 {
            self.write_reg(hart, flash_reg!(modekeyr), FLASH_KEY1);
            self.write_reg(hart, flash_reg!(modekeyr), FLASH_KEY2);
        }

        let final_ctlr = self.read_reg(hart, flash_reg!(ctlr));
        (final_ctlr & (FMC_CTL_LOCK | FMC_CTL_FLOCK)) == 0
    }

    /// Wait until FPEC busy flag is cleared.
    pub fn wait_not_busy(&self, hart: &mut RiscvHart) -> bool {
        for _ in 0..FLASH_TIMEOUT_ITERS {
            let stat = self.read_reg(hart, flash_reg!(statr));
            if (stat & FMC_STAT_BUSY) == 0 {
                return true;
            }
        }
        false
    }

    /// Consume EOP flag and check for write protection errors.
    pub fn check_complete(&self, hart: &mut RiscvHart, _addr: u32) -> bool {
        let stat = self.read_reg(hart, flash_reg!(statr));
        self.write_reg(hart, flash_reg!(statr), FMC_STAT_EOP | FMC_STAT_WRPRTERR);
        (stat & FMC_STAT_WRPRTERR) == 0
    }

    /// Fast page erase (64 B on CH32V003, 256 B on CH32V006).
    pub fn erase(&self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool {
        let page_size = self.info.page_size as usize;
        if !self.unlock(hart) || !self.wait_not_busy(hart) {
            return false;
        }

        let ctlr_saved = self.read_reg(hart, flash_reg!(ctlr));
        self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTER);

        let mut success = true;
        let mut offset = 0;
        while offset < len {
            let page_addr = ((addr + offset as u32) | FLASH_PHYSICAL_ADDRESS) as u32;

            self.write_reg(hart, flash_reg!(addr), page_addr);
            self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTER | FMC_CTL_STRT);

            if !self.wait_not_busy(hart) || !self.check_complete(hart, page_addr) {
                success = false;
                break;
            }
            offset += page_size;
        }

        self.write_reg(hart, flash_reg!(ctlr), ctlr_saved);
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

        let ctlr_saved = self.read_reg(hart, flash_reg!(ctlr));
        let mut success = true;
        let mut offset = 0;

        while offset < len {
            let page_base = ((dest + offset as u32) | FLASH_PHYSICAL_ADDRESS) as u32;

            self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTPG);
            self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTPG | FMC_CTL_BUFRST);
            let _ = self.read_reg(hart, flash_reg!(ctlr));

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

                self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTPG | FMC_CTL_BUFLOAD);
                let _ = self.read_reg(hart, flash_reg!(ctlr));

                if !self.wait_not_busy(hart) {
                    success = false;
                    break;
                }
                curr_dest += 4;
            }

            if !success {
                break;
            }

            self.write_reg(hart, flash_reg!(addr), page_base);
            self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTPG | FMC_CTL_STRT);
            let _ = self.read_reg(hart, flash_reg!(ctlr));

            if !self.wait_not_busy(hart) || !self.check_complete(hart, page_base) {
                success = false;
                break;
            }

            offset += page_size;
        }

        self.write_reg(hart, flash_reg!(ctlr), ctlr_saved);
        success
    }

    /// Load the write stub into target SRAM.
    pub fn prepare_stub(&self, hart: &mut RiscvHart) -> bool {
        hart.mem_write(STUB_CODE_LOCATION, CH32V0X_WRITE_BIN)
            .is_ok()
    }

    /// Does the target's memory still hold `data` at `addr`? (word-wise, so the
    /// comparison needs no buffer as large as the block)
    ///
    /// The debug module's `data0` is clobbered first: an abstract read is "write data1,
    /// write command, read data0", and a read whose command frame is lost hands back
    /// whatever `data0` still holds - i.e. the value just written. Without this priming
    /// read the comparison matches a write that never landed. The prime reads a word
    /// past the end of the stub's data area: inside RAM by the compile-time asserts, and
    /// never part of what is being verified.
    fn mem_matches(&self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        let mut stale = [0u8; 4];
        if hart
            .mem_read(STUB_DATA_LOCATION + STUB_MAX_CHUNK as u32, &mut stale)
            .is_err()
        {
            return false;
        }

        let mut offset = 0usize;
        while offset < data.len() {
            let mut word = [0u8; 4];
            if hart.mem_read(addr + offset as u32, &mut word).is_err() {
                return false;
            }
            let count = core::cmp::min(4, data.len() - offset);
            if word[..count] != data[offset..offset + count] {
                return false;
            }
            offset += count;
        }
        true
    }

    /// High-speed flash programming via in-SRAM flashstub.
    pub fn write_stub(&self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        if data.len() < 256 {
            return self.write_direct(hart, dest, data);
        }

        if !self.unlock(hart) || !self.wait_not_busy(hart) {
            return false;
        }

        let ctlr_saved = self.read_reg(hart, flash_reg!(ctlr));
        let mut remaining = data.len();
        let mut curr_addr = (dest | FLASH_PHYSICAL_ADDRESS) as u32;
        let mut src_offset = 0;
        let page_size = self.info.page_size;

        while remaining > 0 {
            let chunk = remaining.min(STUB_MAX_CHUNK);
            let chunk_data = &data[src_offset..src_offset + chunk];

            if hart.mem_write(STUB_DATA_LOCATION, chunk_data).is_err() {
                self.write_reg(hart, flash_reg!(ctlr), ctlr_saved);
                return false;
            }

            self.write_reg(hart, flash_reg!(ctlr), FMC_CTL_FTPG);

            // The runner's verdict ("halted and reported STUB_EXIT_OK") is not used: see
            // the content check below.
            let _ = hart.run_stub_with_stack_hold(
                STUB_CODE_LOCATION,
                STUB_STACK_LOCATION,
                curr_addr,
                STUB_DATA_LOCATION,
                chunk as u32,
                page_size,
                Some(STUB_RUN_TIMEOUT_MS),
                // `mon stub_hold` replaces the per-page number for a silicon sweep; the
                // scaling with the pages in the block stays the same either way.
                rsbmp::riscv::stub::stub_hold_override_ms()
                    .unwrap_or_else(|| stub_hold_per_page(self.info.family))
                    * (chunk as u32 / page_size),
            );

            // Restore the controller before reading the block back: in fast-program
            // mode the flash is not a normal memory target and a read there returns
            // erased data. The block itself is then the verdict - this part's stub
            // reports failure on runs that did program the page, and reports success
            // on runs that did not, so its own word cannot be the gate.
            self.write_reg(hart, flash_reg!(ctlr), ctlr_saved);
            if !self.mem_matches(hart, curr_addr, chunk_data) {
                // Not programmed: write the block through the register path instead.
                // The range is erased first - flash only clears bits, so a
                // half-programmed page cannot be fixed by programming it again.
                rsbmp::riscv::stub::stub_stat_repair();
                if !self.erase(hart, curr_addr, chunk) {
                    return false;
                }
                return self.write_direct(hart, dest, data);
            }

            curr_addr += chunk as u32;
            src_offset += chunk;
            remaining -= chunk;
        }

        self.write_reg(hart, flash_reg!(ctlr), ctlr_saved);
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
    pub fn page_write(&self, hart: &mut RiscvHart, addr: u32, src: &[u8]) -> bool {
        let page_size = self.info.page_size;
        if src.len() != page_size as usize || (addr & (page_size - 1)) != 0 {
            return false;
        }
        self.write_direct(hart, addr, src)
    }
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () = assert!(core::mem::size_of::<Ch32v00xDriver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for Ch32v00xDriver {
    fn name(&self) -> &'static str {
        self.info.driver_name
    }

    fn part_id(&self) -> u32 {
        self.info.idcode
    }

    fn target_options(&self) -> u32 {
        TOPT_INHIBIT_NRST
    }

    fn has_hw_breakpoints(&self) -> bool {
        self.triggers.has_exec
    }

    fn memory_map(&self) -> MemoryMap {
        let ram_bytes = (self.info.ram_size_kb as usize) * 1024;
        let flash_bytes = (self.info.flash_size_kb as usize) * 1024;

        let mut map = MemoryMap::new().with_ram(RAM_ADDRESS, ram_bytes);

        // The part answers at both the physical flash base and its cached alias, so the
        // debugger may load through either window.
        for start in [FLASH_ALIAS_ADDRESS, FLASH_PHYSICAL_ADDRESS] {
            map = map.with_flash_desc(FlashDesc {
                start,
                length: flash_bytes,
                blocksize: FLASH_BLOCK_SIZE,
                writesize: FLASH_BLOCK_SIZE,
                erasesizes: &ERASE_SIZES,
                erased: 0xff,
            });
        }

        map
    }
}

impl RustRiscvTargetDriver for Ch32v00xDriver {
    fn hw_breakpoint_count(&self) -> u32 {
        self.triggers.exec_slots as u32
    }

    fn flash_prepare(&mut self, hart: &mut RiscvHart, op: FlashOperation) -> bool {
        // Everything a flash session does - the stub upload, the controller pokes, the
        // block read-backs - is an abstract DM access, so it needs the debug module
        // active and the hart halted. BMP's flash-mode entry resets the target, which
        // leaves the DM inactive; without this, the very first access of the session
        // (the stub upload below) silently fails and the erase is reported as broken.
        if !hart.dm_activate() || hart.halt_request().is_err() {
            return false;
        }

        // No engine to keep: this family's controller keeps no save-area of its own (the
        // probe results are all the driver holds), the session state is the stub in SRAM.
        //
        // Erase pokes the controller's page erase directly, so only the programming stub is a
        // resident capability: an erase in the same session drops the flag (`StubCaps::ERASE`
        // is not advertised) and the next write uploads again.
        if !self.stubs.needs_upload(op) {
            return true;
        }
        if !self.prepare_stub(hart) {
            return false;
        }
        self.stubs.uploaded();
        true
    }

    fn flash_done(&mut self, _hart: &mut RiscvHart) -> bool {
        // The session handed the target back: the RAM the stub lives in is the application's.
        self.stubs.invalidate();
        true
    }

    fn flash_erase(&mut self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool {
        self.erase(hart, addr, len)
    }

    fn flash_write(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        let ok = self.write_stub(hart, dest, data);
        if !ok {
            // A failed run can leave the core mid-image: the next prepare uploads again.
            self.stubs.invalidate();
        }
        ok
    }

    fn sw_page_size(&self) -> u32 {
        self.info.page_size
    }

    fn sw_page_erase(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        self.page_erase(hart, addr)
    }

    fn sw_page_write(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        self.page_write(hart, addr, data)
    }
}
