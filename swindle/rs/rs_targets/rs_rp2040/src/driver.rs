//! RP2040 (Raspberry Pi Pico 1) driver.
//!
//! Rust port of `blackmagic/src/target/rp2040.c`: `rp2040_probe()`, `rp_attach()`, the
//! boot ROM function table lookup, the memory map and the flash/CRC hooks. The flash
//! controller itself lives in [`crate::flash`], the `mon` commands in
//! [`crate::mon_commands`].

use crate::crc32;
use crate::flash::{self, SsiEngine, SsiState};
use crate::mon_commands;
use crate::registers::*;
use rs_rp_common::spi::SpiFlash;
use rsbmp::bmplogger::G;
use rsbmp::commands::CommandTree;
use rsbmp::flash::{FlashOperation, StubCaps, StubState};
use rsbmp::format_util::HexCase;
use rsbmp::gdb_println;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_INHIBIT_NRST,
};

/// Bytes of the boot ROM function table the driver reads (C `RP_MAX_TABLE_SIZE`).
const ROM_TABLE_BYTES: usize = MAX_TABLE_SIZE;

/// One entry of that table: a two-character tag and a 16-bit ROM address.
const ROM_ENTRY_BYTES: usize = 4;

/// Driver name as the C `target_s` carries it.
const DRIVER_NAME: &str = "RP2040\0";

/// What a `mon` command works with: the flash region and the ROM entry point.
///
/// Built on demand from the live driver by [`active_target()`], so the probe results exist
/// in one place only — the driver itself.
#[derive(Debug, Clone, Copy)]
pub struct Rp2040Target {
    /// Flash region the probe discovered.
    pub flash: SpiFlash,
    /// ROM entry point that reboots into BOOTSEL mode, absent when the lookup failed.
    pub rom_reset_usb_boot: Option<u16>,
}

/// The probed target, if this crate's driver is the one attached.
///
/// The `mon` callbacks get no driver handle (the C trampoline passes only the packet text),
/// so they resolve the live driver in the ARM dispatcher and project the two probe results
/// they need out of it. Nothing is stored here between commands.
pub fn active_target() -> Option<Rp2040Target> {
    rsbmp::arm::driver_registry::active_arm_driver_as::<Rp2040Driver>()
        .map(Rp2040Driver::target)
}

/// The RP2040 driver instance for one probed part.
///
/// The part is what the probe *found*; the SSI is only in the driver while a flash session
/// holds it ([`SsiEngine`]), which is why the erase/write hooks go through that field instead
/// of reaching for the controller directly.
#[derive(Debug, Clone, Copy)]
pub struct Rp2040Driver {
    /// Flash region the probe discovered.
    flash: SpiFlash,
    /// ROM entry point that reboots into BOOTSEL mode.
    rom_reset_usb_boot: Option<u16>,
    /// The SSI while `flash_prepare()`..`flash_done()` holds it, `None` between sessions.
    session: Option<SsiEngine>,
    /// Whether the write flashstub is currently parked in SRAM.
    stubs: StubState,
}

impl Rp2040Driver {
    /// The probe results a `mon` command needs, as the view [`active_target()`] hands out.
    pub fn target(&self) -> Rp2040Target {
        Rp2040Target {
            flash: self.flash,
            rom_reset_usb_boot: self.rom_reset_usb_boot,
        }
    }
}

/// Probe an RP2040 (C `rp2040_probe()`).
pub fn probe(cm: &mut CortexMAccess) -> Option<Rp2040Driver> {
    // The part number has to match before anything else is worth looking at.
    if cm.part_id() != PART_ID {
        return None;
    }

    // The boot ROM magic rules out a chip that merely shares the part number.
    let boot_magic = cm.read32(BOOTROM_MAGIC_ADDR);
    if boot_magic & BOOTROM_MAGIC_MASK != BOOTROM_MAGIC {
        G::print_str("Wrong Bootmagic ");
        G::print_hex_padded(boot_magic, 8, HexCase::Lower);
        G::print_str(" found!\n");
        return None;
    }
    if boot_magic >> BOOTROM_VERSION_SHIFT == 1 {
        gdb_println!("Old Bootrom Version 1!");
    }

    // The C driver does this from its attach hook (`rp_attach()`), i.e. with the core
    // halted. Doing it here is what lets the memory map carry the real flash size, so the
    // core is halted around the discovery and resumed afterwards when it was running:
    // a core executing from flash faults while the controller is in direct mode.
    let (rom_reset_usb_boot, flash) = cm.with_halted(|cm| {
        let rom_reset_usb_boot = read_rom_func_table(cm);
        // Scratch only: `add_flash()` takes the SSI over and puts it back itself.
        let mut state = SsiState::default();
        let flash = flash::add_flash(cm, &mut state);
        (rom_reset_usb_boot, flash)
    });

    Some(Rp2040Driver {
        flash,
        rom_reset_usb_boot,
        session: None,
        stubs: StubState::new(StubCaps::WRITE),
    })
}

/// Read the boot ROM function table and pick out the "reset into USB boot" entry
/// (C `rp_read_rom_func_table()`).
///
/// Entries are pairs of 16-bit integers: a two-character tag and the ROM address of the
/// routine (datasheet section 2.8.3).
fn read_rom_func_table(cm: &CortexMAccess) -> Option<u16> {
    // The pointer stored in the header is only 16 bits wide.
    let table_offset = cm.read32(BOOTROM_FUNC_TABLE_ADDR) as u16;
    let mut table = [0u8; ROM_TABLE_BYTES];
    cm.mem_read(&mut table, u32::from(table_offset));
    if cm.check_error() {
        return None;
    }

    for entry in table.chunks_exact(ROM_ENTRY_BYTES) {
        let tag = u16::from_le_bytes([entry[0], entry[1]]);
        if tag == BOOTROM_FUNC_TABLE_TAG_UB {
            return Some(u16::from_le_bytes([entry[2], entry[3]]));
        }
    }
    None
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () = assert!(core::mem::size_of::<Rp2040Driver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for Rp2040Driver {
    fn name(&self) -> &'static str {
        DRIVER_NAME
    }

    fn part_id(&self) -> u32 {
        PART_ID
    }

    fn target_options(&self) -> u32 {
        // The part does not survive the reset line being pulsed.
        TOPT_INHIBIT_NRST
    }

    fn memory_map(&self) -> MemoryMap {
        // Program in the largest whole-page unit the flash manager's fixed buffer can hold
        // instead of in block units like the C driver: the C manager sizes its buffer from
        // the geometry with `malloc`, while Swindle's is a static
        // `MAX_FLASH_BUFFER_SIZE` bytes and cannot hold a 4KiB block. The stub underneath
        // splits the unit into page programs itself.
        MemoryMap::new()
            .with_flash(
                XIP_FLASH_BASE,
                self.flash.length,
                self.flash.sector_size,
                rsbmp::flash::buffered_write_unit(self.flash.page_size),
            )
            .with_ram(SRAM_BASE, SRAM_SIZE)
    }

    fn commands(&self) -> &'static [CommandTree] {
        &mon_commands::COMMANDS
    }
}

impl RustArmTargetDriver for Rp2040Driver {
    fn flash_prepare(&mut self, cm: &mut CortexMAccess, op: FlashOperation) -> bool {
        // An engine is kept only when the controller really was taken over: a failed prepare
        // is rolled back by the flash manager without a matching `flash_done()`.
        //
        // The write stub is uploaded once per session: erase goes through the SSI directly, so
        // only programming needs it (`StubCaps::ERASE` is not advertised and an erase in the
        // same session drops the flag).
        let load_stub = self.stubs.needs_upload(op);
        self.session = SsiEngine::engage(cm, self.flash, load_stub);
        if self.session.is_some() && load_stub {
            self.stubs.uploaded();
        }
        self.session.is_some()
    }

    fn flash_done(&mut self, cm: &mut CortexMAccess) -> bool {
        if let Some(engine) = self.session.take() {
            engine.release(cm);
        }
        // `release()` restarts the part, so the stub is gone from SRAM for good.
        self.stubs.invalidate();
        true
    }

    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, _len: usize) -> bool {
        // Only the block holding `addr` has to be erased: the flash manager calls this once
        // per block. Without a session the SSI is not ours to drive.
        match self.session.as_mut() {
            Some(engine) => engine.erase(cm, addr),
            None => false,
        }
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        let ok = match self.session.as_mut() {
            Some(engine) => engine.write(cm, dest, data),
            None => false,
        };
        if !ok {
            // A failed run can leave the core mid-image: the next prepare uploads again.
            self.stubs.invalidate();
        }
        ok
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        // The target-level mass erase runs outside a prepared session (C
        // `target_flash_mass_erase()` calls `target_s.mass_erase` directly), so the SSI is
        // taken over for the duration when no session holds it. Mass erase does not use the
        // stub, so nothing is uploaded.
        match self.session.as_mut() {
            Some(engine) => engine.mass_erase(cm),
            None => match SsiEngine::engage(cm, self.flash, false) {
                Some(mut engine) => {
                    let erased = engine.mass_erase(cm);
                    engine.release(cm);
                    erased
                }
                None => false,
            },
        }
    }

    fn crc32(&mut self, cm: &mut CortexMAccess, addr: u32, len: u32) -> Option<u32> {
        crc32::crc32(cm, addr, len as usize)
    }

    fn attach(&mut self, cm: &mut CortexMAccess) -> bool {
        // C `rp_attach()`: attach to the core, then require the ROM table lookup that
        // `mon reset_usb_boot` needs.
        if !cm.cortexm_attach() {
            return false;
        }
        self.rom_reset_usb_boot.is_some()
    }
}
