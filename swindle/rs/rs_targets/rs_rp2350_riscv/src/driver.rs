//! RP2350 driver for its Hazard3 RISC-V cores.
//!
//! The RISC-V half of `blackmagic/src/target/rp2350.c`: `rp2350_probe()`, the memory map,
//! the flash/CRC hooks and the flash-page breakpoint helpers. The QMI controller itself is
//! in [`rs_rp2350_common::flash`], shared with the Cortex-M33 half in `rs_rp2350_arm`.
//!
//! The C probe tells the two halves apart by the part number the scan filled in
//! (`ID_RP2350_ARM` / `ID_RP2350_RISCV`). A Rust RISC-V driver is only handed the hart, and
//! this one is probed for every RISC-V core whose designer code points at someone else, so
//! it identifies the chip by the boot ROM magic instead: that also rejects an RP2040, whose
//! magic carries version 1 where the RP2350 has 2.

use rs_rp2350_common::flash::{self, Qmi, QmiEngine};
use rs_rp2350_common::flashstub::RP2350_WRITE_RISCV_BIN;
use rs_rp2350_common::registers::*;
use rs_rp_common::access::HartAccess;
use rs_rp_common::spi::SpiFlash;
use rsbmp::flash::{FlashOperation, StubCaps, StubState};
use rsbmp::riscv::{HwTriggerSupport, HwTriggers, RiscvHart};
use rsbmp::target::{MemoryMap, PartDescriptor, RustRiscvTargetDriver};

/// Driver name as the C `target_s` carries it.
const DRIVER_NAME: &str = "RP2350\0";

/// The RP2350 driver instance for one probed part.
///
/// The part is what the probe *found*; the QMI is only in the driver while a flash session
/// holds it ([`QmiEngine`]), which is why the erase/write hooks go through that field instead
/// of reaching for the controller directly. The memory map is computed from these on demand
/// ([`RustRiscvTargetDriver::memory_map`]), so no region descriptors are stored here.
#[derive(Debug, Clone, Copy)]
pub struct Rp2350RiscvDriver {
    /// Flash region the probe discovered, absent when no chip answered.
    flash: Option<SpiFlash>,
    /// Hardware breakpoint capability, resolved at probe time by [`HwTriggers`].
    triggers: HwTriggerSupport,
    /// The QMI while `flash_prepare()`..`flash_done()` holds it, `None` between sessions.
    session: Option<QmiEngine>,
    /// Whether the programming flashstub is currently parked in SRAM.
    stubs: StubState,
}

/// Probe an RP2350 Hazard3 core (C `rp2350_probe()`).
pub fn probe(hart: &mut RiscvHart) -> Option<Rp2350RiscvDriver> {
    // The boot ROM magic is what tells a real RP2350 from a chip at the same addresses.
    let boot_magic = hart.mem_read32(BOOTROM_MAGIC).ok()?;
    if boot_magic & BOOTROM_MAGIC_MASK != BOOTROM_MAGIC_VALUE {
        return None;
    }

    // Do not probe flash here: the core may be actively running code from XIP flash.
    // Probing flash takes over QMI, cycles pad pull-ups and sends exit-QPI (0xf5),
    // which crashes running cores. Flash detection is deferred to flash_prepare().
    Some(Rp2350RiscvDriver {
        flash: None,
        triggers: HwTriggers::Discover.resolve(hart),
        session: None,
        stubs: StubState::new(StubCaps::WRITE),
    })
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () =
    assert!(core::mem::size_of::<Rp2350RiscvDriver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for Rp2350RiscvDriver {
    fn name(&self) -> &'static str {
        DRIVER_NAME
    }

    fn part_id(&self) -> u32 {
        PART_ID_RISCV
    }

    fn has_hw_breakpoints(&self) -> bool {
        self.triggers.has_exec
    }

    fn memory_map(&self) -> MemoryMap {
        let mut map = MemoryMap::new().with_ram(SRAM_BASE, SRAM_SIZE);
        let (flash_len, sector_size, page_size) = match self.flash {
            Some(flash) => (flash.length, flash.sector_size, flash.page_size),
            None => (4 * 1024 * 1024, 4096, 256),
        };
        map = map.with_flash(
            XIP_FLASH_BASE,
            flash_len,
            sector_size,
            rsbmp::flash::buffered_write_unit(page_size),
        );
        map
    }
}

impl RustRiscvTargetDriver for Rp2350RiscvDriver {
    fn hw_breakpoint_count(&self) -> u32 {
        u32::from(self.triggers.exec_slots)
    }

    fn flash_prepare(&mut self, hart: &mut RiscvHart, op: FlashOperation) -> bool {
        let m = HartAccess::new(hart);
        if self.flash.is_none() {
            self.flash = flash::add_flash(&m);
        }
        // An engine is kept only when a chip answered: a part without one has nothing to hand
        // over, and the flash manager reports the failed session itself. A failed prepare is
        // rolled back without a matching `flash_done()`.
        self.session = self.flash.map(|flash| QmiEngine::engage(&m, flash));
        if self.session.is_none() {
            return false;
        }
        // Block erase goes through the QMI, so only programming needs the stub: one upload per
        // session instead of one per write burst.
        if !self.stubs.needs_upload(op) {
            return true;
        }
        if hart
            .mem_write(STUB_CODE_LOCATION_WRITE, RP2350_WRITE_RISCV_BIN)
            .is_err()
        {
            return false;
        }
        self.stubs.uploaded();
        true
    }

    fn flash_done(&mut self, hart: &mut RiscvHart) -> bool {
        let m = HartAccess::new(hart);
        if let Some(engine) = self.session.take() {
            engine.release(&m);
        }
        // The session handed the target back: the RAM the stub lives in is the application's.
        self.stubs.invalidate();
        true
    }

    fn flash_erase(&mut self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool {
        // Only the block holding `addr` has to be erased: the flash manager calls this once
        // per block. Without a session the QMI is not ours to drive.
        let m = HartAccess::new(hart);
        match self.session.as_mut() {
            Some(engine) => {
                let ok = engine.erase(&m, addr);
                if ok {
                    flash::flush_cache(&m, addr, len);
                }
                ok
            }
            None => false,
        }
    }

    fn flash_write(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool {
        if self.session.is_none() {
            return false;
        }
        let mut offset = 0;
        let mut remaining = data.len();
        let mut cur_dest = dest;

        while remaining > 0 {
            let chunk = remaining.min(ATOMIC_FLASHSTUB_WRITE);
            let slice = &data[offset..offset + chunk];

            if hart.mem_write(STUB_DATA_LOCATION, slice).is_err() {
                return false;
            }
            let ok = hart.run_stub_with_stack(
                STUB_CODE_LOCATION_WRITE,
                STUB_STACKEND_LOCATION,
                cur_dest,
                STUB_DATA_LOCATION,
                chunk as u32,
                0,
            );
            if !ok {
                // A failed run can leave the core mid-image: the next prepare uploads again.
                self.stubs.invalidate();
                return false;
            }
            let m = HartAccess::new(hart);
            flash::flush_cache(&m, cur_dest, chunk);

            offset += chunk;
            cur_dest += chunk as u32;
            remaining -= chunk;
        }
        true
    }

    fn crc32(&mut self, _hart: &mut RiscvHart, _addr: u32, _len: u32) -> Option<u32> {
        // `crate::crc32` is kept working against the Hazard3 stub (whose entry point now sets
        // the common exit code in `a0` after publishing the checksum in `a2`); enabling it is a
        // matter of returning its result here once the hardware run has been validated.
        None
    }

    /// A software breakpoint in flash patches the whole erase block, so the page the C
    /// target layer hands around is a sector.
    fn sw_page_size(&self) -> u32 {
        self.flash.map_or(0, |flash| flash.sector_size as u32)
    }

    fn sw_page_erase(&mut self, hart: &mut RiscvHart, addr: u32) -> bool {
        let Some(flash) = self.flash else {
            return false;
        };
        let m = HartAccess::new(hart);
        // Take the QMI over and hand it back without the reset `flash_done()` does: this
        // runs while GDB inserts a breakpoint, and the core has to stay where it is.
        let switched = flash::prepare(&m);
        let erased = flash.erase(&m, &Qmi, addr);
        if switched {
            flash::spi_resume(&m);
        }
        erased
    }

    fn sw_page_write(&mut self, hart: &mut RiscvHart, addr: u32, data: &[u8]) -> bool {
        let Some(flash) = self.flash else {
            return false;
        };
        let m = HartAccess::new(hart);
        let switched = flash::prepare(&m);
        let written = flash.write(&m, &Qmi, addr, data);
        if switched {
            flash::spi_resume(&m);
        }
        written
    }
}
