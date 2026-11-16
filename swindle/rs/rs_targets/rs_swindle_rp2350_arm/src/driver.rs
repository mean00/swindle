/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust port of `blackmagic/src/target/rp2350.c` (`rp2350_probe()`, `rp2350_attach()` and
 * the memory map), made for the Swindle firmware. This file is derived from that C source
 * and stays under the same license as the original Black Magic code reproduced below.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

//! RP2350 driver.
//!
//! Rust port of `blackmagic/src/target/rp2350.c`: `rp2350_probe()`, `rp2350_attach()`,
//! the memory map and the flash/CRC hooks. The QMI controller lives in
//! [`rs_swindle_rp2350_common::flash`].
//!
//! The Cortex-M33 half of the chip. The Hazard3 RISC-V cores carry the same part numbers and
//! answer the same probe test, but they are driven through `rsbmp::riscv` and have their own
//! driver crate (`rs_swindle_rp2350_riscv`): this one is what the probe list of `cortexm.c` reaches.

use rs_swindle_rp2350_common::flash::{self, QmiEngine};
use rs_swindle_rp2350_common::flashstub::RP2350_WRITE_ARM_BIN;
use rs_swindle_rp2350_common::registers::*;
use rs_swindle_rp_common::spi::SpiFlash;
use rsbmp::bmplogger::G;
use rsbmp::flash::{FlashOperation, StubCaps, StubState};
use rsbmp::format_util::HexCase;
use rsbmp::target::{CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TargetView};

/// Driver name as the C `target_s` carries it.
const DRIVER_NAME: &str = "RP2350\0";

/// How many times [`Rp2350Driver::flash_prepare`] polls for the halt request to land before
/// going on. `halt_poll()` is itself bounded, so this only covers a core that is slow to stop.
const HALT_SETTLE_POLLS: u32 = 100;

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
    /// Whether the programming flashstub is parked in SRAM and usable.
    stubs: StubState,
    /// Set when a stub run failed: the rest of the session programs through the QMI FIFO.
    qmi_fallback: bool,
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
    //
    // The C driver also overrides `enter_flash_mode`/`exit_flash_mode` with
    // `rp2350_flash_prepare()`/`rp2350_flash_resume()`: entering flash mode must take the QMI
    // into direct mode, not reset the part. With the generic reset in place the core is
    // released into the ROM bootloader, which then drives the QMI/pads underneath us, the
    // core no longer halts on request, the JEDEC/SFDP reads come back garbage and the
    // flashstub upload fails. The QMI is taken in [`Rp2350Driver::flash_prepare`] and handed
    // back in [`Rp2350Driver::flash_done`], so the generic reset is exactly what must not run.
    TargetView::new(cm.target()).set_enter_flash_mode_stub();
    Some(Rp2350Driver {
        part_id,
        flash: None,
        session: None,
        stubs: StubState::new(StubCaps::WRITE),
        qmi_fallback: false,
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
        // The QMI and the QSPI pads are shared with the core: a core executing from XIP
        // faults as soon as the controller goes into direct mode, and the boot ROM (or the
        // application) would otherwise drive the pads underneath us. GDB keeps the core
        // halted across a `load`, but a `mon erase`/`mon …` flash operation can arrive with
        // the core running, so make sure it is stopped for the session.
        if !cm.halted() {
            cm.halt_request();
            for _ in 0..HALT_SETTLE_POLLS {
                if cm.halted() {
                    break;
                }
            }
        }
        if self.session.is_none() {
            // Fresh session: put the core back at the reset vector (Secure Privileged, QSPI
            // pads in their POR state) so the flashstub can drive the QMI, and re-detect the
            // chip from that known state.
            cm.reset_and_halt();
            self.stubs.invalidate();
            self.qmi_fallback = false;
            self.flash = flash::add_flash(cm);
        }
        self.session = self.flash.map(|flash| QmiEngine::engage(cm, flash));
        if self.session.is_none() {
            return false;
        }
        // The stub is what makes programming fast (4 KiB chunks instead of two bytes per AP
        // write); it is uploaded once per session. Erase does not need it, but the upload is
        // 358 bytes and it keeps a `load` (erase then write) on the stub throughout.
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
        // The RAM the stub sits in belongs to the application again.
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
        // The fast path is the flashstub: 4 KiB chunks programmed by the core, which is what
        // makes this comparable to the RP2040 driver. It needs a Secure Privileged core, which
        // `flash_prepare()` arranges with `reset_to_halt()`.
        //
        // If a run does not report success (the core could not reach the QMI, or the run was
        // interrupted), the rest of the session falls back to streaming the data out of the
        // QMI transmit FIFO from the probe: slower, but independent of the core's state.
        // `QmiEngine` is `Copy`, so taking it out keeps `self` free for the flag.
        let Some(mut engine) = self.session else {
            return false;
        };
        let mut offset = 0;
        let mut remaining = data.len();
        let mut cur_dest = dest;

        while remaining > 0 {
            let chunk = remaining.min(ATOMIC_FLASHSTUB_WRITE);
            let slice = &data[offset..offset + chunk];

            if !self.qmi_fallback {
                cm.mem_write(STUB_DATA_LOCATION, slice);
                let ran = cm.run_stub_with_stack(
                    STUB_CODE_LOCATION_WRITE,
                    STUB_STACKEND_LOCATION,
                    cur_dest,
                    STUB_DATA_LOCATION,
                    chunk as u32,
                    0,
                );
                if ran {
                    offset += chunk;
                    cur_dest += chunk as u32;
                    remaining -= chunk;
                    continue;
                }
                // A failed run can leave the core mid-image: the next session uploads again,
                // and this one finishes through the controller.
                self.stubs.invalidate();
                self.qmi_fallback = true;
            }
            if !engine.write(cm, cur_dest, slice) {
                return false;
            }
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
