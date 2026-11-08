//! RP2350 driver.
//!
//! Rust port of `blackmagic/src/target/rp2350.c`: `rp2350_probe()`, `rp2350_attach()`,
//! the memory map and the flash/CRC hooks. The QMI controller lives in
//! [`rs_rp2350_common::flash`].
//!
//! The Cortex-M33 half of the chip. The Hazard3 RISC-V cores carry the same part numbers and
//! answer the same probe test, but they are driven through `rsbmp::riscv` and have their own
//! driver crate (`rs_rp2350_riscv`): this one is what the probe list of `cortexm.c` reaches.

use rs_rp2350_common::flash::{self, QmiEngine};
use rs_rp2350_common::flashstub::RP2350_WRITE_ARM_BIN;
use rs_rp2350_common::registers::*;
use rs_rp_common::spi::SpiFlash;
use rsbmp::bmplogger::G;
use rsbmp::flash::{FlashOperation, StubCaps, StubState};
use rsbmp::format_util::HexCase;
use rsbmp::target::{CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver};

/// Driver name as the C `target_s` carries it.
const DRIVER_NAME: &str = "RP2350\0";

/// The RP2350 driver instance for one probed part.
///
/// The part is what the probe *found*; the QMI is only in the driver while a flash session
/// holds it ([`QmiEngine`]), which is why the erase/write hooks go through that field instead
/// of reaching for the controller directly.
#[derive(Debug, Clone, Copy)]
pub struct Rp2350Driver {
    /// Part number the probe found, for the C `target_s`.
    part_id: u32,
    /// Flash region the probe discovered, absent when no chip answered.
    flash: Option<SpiFlash>,
    /// The QMI while `flash_prepare()`..`flash_done()` holds it, `None` between sessions.
    session: Option<QmiEngine>,
    /// Whether the programming flashstub is currently parked in SRAM.
    stubs: StubState,
}

/// Probe an RP2350 (C `rp2350_probe()`).
pub fn probe(cm: &mut CortexMAccess) -> Option<Rp2350Driver> {
    let part_id = cm.part_id();
    // Both the Arm and the Hazard3 cores carry these part numbers.
    if part_id != PART_ID_ARM && part_id != PART_ID_RISCV {
        return None;
    }

    // The boot ROM magic is what tells a real RP2350 from a chip sharing the part number.
    let boot_magic = cm.read32(BOOTROM_MAGIC);
    if boot_magic & BOOTROM_MAGIC_MASK != BOOTROM_MAGIC_VALUE {
        G::print_str("Wrong Bootmagic ");
        G::print_hex_padded(boot_magic, 8, HexCase::Lower);
        G::print_str(" found!\n");
        return None;
    }

    // Do not probe flash here: the core may be actively running code from XIP flash.
    // Probing flash takes over QMI, cycles pad pull-ups and sends exit-QPI (0xf5),
    // which crashes running cores. Flash detection is deferred to attach() or flash_prepare().
    Some(Rp2350Driver {
        part_id,
        flash: None,
        session: None,
        stubs: StubState::new(StubCaps::WRITE),
    })
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () = assert!(core::mem::size_of::<Rp2350Driver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for Rp2350Driver {
    fn name(&self) -> &'static str {
        DRIVER_NAME
    }

    fn part_id(&self) -> u32 {
        self.part_id
    }

    fn memory_map(&self) -> MemoryMap {
        let mut map = MemoryMap::new();
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
        map.with_ram(SRAM_BASE, SRAM_SIZE)
    }
}

impl RustArmTargetDriver for Rp2350Driver {
    fn attach(&mut self, cm: &mut CortexMAccess) -> bool {
        cm.cortexm_attach()
    }

    fn flash_prepare(&mut self, cm: &mut CortexMAccess, op: FlashOperation) -> bool {
        if self.flash.is_none() {
            self.flash = flash::add_flash(cm);
        }
        self.session = self.flash.map(|flash| QmiEngine::engage(cm, flash));
        if self.session.is_none() {
            return false;
        }
        // Block erase goes through the QMI, so only programming needs the stub: one upload per
        // session instead of one per write burst.
        if !self.stubs.needs_upload(op) {
            return true;
        }
        cm.mem_write(STUB_CODE_LOCATION_WRITE, RP2350_WRITE_ARM_BIN);
        if cm.check_error() {
            return false;
        }
        self.stubs.uploaded();
        true
    }

    fn flash_done(&mut self, cm: &mut CortexMAccess) -> bool {
        if let Some(engine) = self.session.take() {
            engine.release(cm);
        }
        flash::flush_cache(cm, 0, 0);
        // The session handed the target back: the RAM the stub lives in is the application's.
        self.stubs.invalidate();
        true
    }

    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, len: usize) -> bool {
        // Only the block holding `addr` has to be erased: the flash manager calls this once
        // per block. Without a session the QMI is not ours to drive.
        match self.session.as_mut() {
            Some(engine) => {
                let ok = engine.erase(cm, addr);
                if ok {
                    flash::flush_cache(cm, addr, len);
                }
                ok
            }
            None => false,
        }
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        if self.session.is_none() {
            return false;
        }
        let mut offset = 0;
        let mut remaining = data.len();
        let mut cur_dest = dest;

        while remaining > 0 {
            let chunk = remaining.min(ATOMIC_FLASHSTUB_WRITE);
            let slice = &data[offset..offset + chunk];

            cm.mem_write(STUB_DATA_LOCATION, slice);
            let ok = cm.run_stub_with_stack(
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
            flash::flush_cache(cm, cur_dest, chunk);

            offset += chunk;
            cur_dest += chunk as u32;
            remaining -= chunk;
        }
        true
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        // The target-level mass erase runs outside a prepared session, so the QMI is taken
        // over for the duration when no session holds it.
        match self.session.as_mut() {
            Some(engine) => engine.mass_erase(cm),
            None => match self.flash {
                Some(flash) => {
                    let mut engine = QmiEngine::engage(cm, flash);
                    let erased = engine.mass_erase(cm);
                    engine.release(cm);
                    erased
                }
                None => false,
            },
        }
    }

    fn crc32(&mut self, _cm: &mut CortexMAccess, _addr: u32, _len: u32) -> Option<u32> {
        // Fall back to Swindle's verified do_local_crc32 to match GDB compare-sections.
        // `crate::crc32` is kept working against the stub (which now publishes the common exit
        // code, `bkpt #1`); enabling it is a matter of returning its result here once the
        // hardware run has been validated.
        None
    }
}
