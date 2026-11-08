//! The RP2040 flash controller: SSI in plain SPI mode, XIP entry/exit, the write stub.
//!
//! Rust port of the flash half of `blackmagic/src/target/rp2040.c` (`rp_add_flash()`,
//! `rp_flash_prepare()`/`_resume()`, the `rp_flash_*` boot ROM sequences, the SSI access
//! helpers and `rp_flash_write()`), plus the `Ssi` transport the shared flash layer
//! ([`rs_rp_common::spi`]) drives.
//!
//! The RP2040 boots with the XIP controller presenting the QSPI flash at
//! `0x10000000`. To program it the driver takes the SSI controller over, drives the chip
//! directly and puts the XIP mode back afterwards. Programming itself is done by a small
//! stub in SRAM: the chunk goes ahead of it and the stub talks to the SSI, so the debug
//! link only has to carry the data once.

use crate::registers::*;
use rs_rp_common::spi::{self, SpiFlash, SpiTransport};
use rs_rp_common::access::FlashAccess;
use rsbmp::target::{CortexMAccess, delay_ms};

/// Data bytes the SSI may have in flight before it overflows its RX FIFO, minus the two
/// the peripheral keeps internally (C `rp_flash_put_get()`'s `max_in_flight`).
const MAX_IN_FLIGHT: u32 = 16 - 2;

/// Settling time for the pad pulls during the exit-XIP sequence, in milliseconds
/// (C `platform_delay(10)`).
const PULL_SETTLE_MS: u32 = 10;

/// SSI clock divider used while the chip is under direct control: a conservative rate
/// for boot and programming (C `rp_flash_init_spi()`).
const SSI_BAUD_DIVIDER: u32 = 6;

/// Command byte used for XIP reads once the flash is handed back (C `0x03`, "read").
const XIP_READ_COMMAND: u32 = 0x03;
/// Address bytes of that command.
const XIP_READ_ADDRESS_BYTES: u32 = 3;

/// The SSI state the driver changes and has to put back (C `rp_priv_s`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SsiState {
    /// Value of [`SSI_ENABLE`] before the driver took the controller over.
    pub enabled: u32,
    /// Value of [`SSI_CTRL0`].
    pub ctrl0: u32,
    /// Value of [`SSI_CTRL1`].
    pub ctrl1: u32,
    /// Value of [`SSI_XIP_SPI_CTRL0`].
    pub xpi_ctrl0: u32,
}

/// The SSI driven as a plain, 8-bit serial SPI controller.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ssi;

impl SpiTransport for Ssi {
    fn read(&self, cm: &dyn FlashAccess, command: u16, address: u32, buffer: &mut [u8]) {
        setup_xfer(cm, command, address);
        for byte in buffer.iter_mut() {
            *byte = xfer_data(cm, 0);
        }
        chip_select(cm, GPIO_QSPI_CS_DRIVE_HIGH);
    }

    fn write(&self, cm: &dyn FlashAccess, command: u16, address: u32, buffer: &[u8]) {
        setup_xfer(cm, command, address);
        for &byte in buffer {
            xfer_data(cm, byte);
        }
        chip_select(cm, GPIO_QSPI_CS_DRIVE_HIGH);
    }

    fn run_command(&self, cm: &dyn FlashAccess, command: u16, address: u32) {
        setup_xfer(cm, command, address);
        chip_select(cm, GPIO_QSPI_CS_DRIVE_HIGH);
    }
}

/// Drive the QSPI chip select (C `rp_spi_chip_select()`).
fn chip_select(cm: &dyn FlashAccess, state: u32) {
    let value = cm.read32(gpio_qspi_reg!(cs_ctrl));
    cm.write32(gpio_qspi_reg!(cs_ctrl), (value & !GPIO_QSPI_CS_DRIVE_MASK) | state);
}

/// Clock one byte through the SSI, returning what came back (C `rp_spi_xfer_data()`).
fn xfer_data(cm: &dyn FlashAccess, data: u8) -> u8 {
    cm.write32(ssi_reg!(dr0), u32::from(data));
    cm.read32(ssi_reg!(dr0)) as u8
}

/// Start a transaction: select the chip and send the command, address and dummy bytes
/// (C `rp_spi_setup_xfer()`).
fn setup_xfer(cm: &dyn FlashAccess, command: u16, address: u32) {
    chip_select(cm, GPIO_QSPI_CS_DRIVE_LOW);

    xfer_data(cm, (command & spi::OPCODE_MASK) as u8);
    if command & spi::OPCODE_MODE_MASK == spi::OPCODE_3B_ADDR {
        // For each byte sent here, the controller has to be drained with a read.
        xfer_data(cm, (address >> 16) as u8);
        xfer_data(cm, (address >> 8) as u8);
        xfer_data(cm, address as u8);
    }

    let dummy_bytes = usize::from((command & spi::DUMMY_MASK) >> spi::DUMMY_SHIFT);
    for _ in 0..dummy_bytes {
        xfer_data(cm, 0);
    }
}


/// Whether the QSPI and XIP controllers still need configuring.
///
/// The C `rp_flash_in_por_state()`: true when the boot ROM has not brought the flash
/// clocks up, or when the pads, the XIP block or the SSI still look like reset leaves them.
pub fn needs_configuring(cm: &dyn FlashAccess) -> bool {
    let clocks0 = cm.read32(clocks_reg!(wake_en0));
    let clocks1 = cm.read32(clocks_reg!(wake_en1));
    if clocks0 & CLOCKS_WAKE_EN0_MASK != CLOCKS_WAKE_EN0_MASK
        || clocks1 & CLOCKS_WAKE_EN1_MASK != CLOCKS_WAKE_EN1_MASK
    {
        // Enable all the clocks the boot ROM does before it touches the flash.
        cm.write32(clocks_reg!(wake_en0), 0xffff_ffff);
        cm.write32(clocks_reg!(wake_en1), 0xffff_ffff);
        return true;
    }
    // Input enabled on the clock pad means the pads were never muxed to the QSPI block.
    if cm.read32(pads_qspi_reg!(gpio_sclk)) & PADS_QSPI_GPIO_SCLK_IE != 0 {
        return true;
    }
    cm.read32(xip_reg!(stat)) == XIP_STAT_POR
        && cm.read32(gpio_qspi_reg!(sclk_ctrl)) == GPIO_QSPI_SCLK_POR
        && cm.read32(ssi_reg!(enable)) == 0
}

/// Connect the XIP controller to the QSPI pads, from a known reset state
/// (C `rp_flash_connect_internal()`).
fn connect_internal(cm: &dyn FlashAccess) {
    let io_pads_bits = RESETS_RESET_IO_QSPI_BITS | RESETS_RESET_PADS_QSPI_BITS;
    // A hard reset forces the IO and pad controls into a known state. IO_BANK0 is left
    // alone as it does not affect the XIP signals.
    cm.write32(resets_reg!(reset) | REG_ACCESS_WRITE_ATOMIC_BITSET, io_pads_bits);
    cm.write32(resets_reg!(reset) | REG_ACCESS_WRITE_ATOMIC_BITCLR, io_pads_bits);
    let mut reset_done = 0;
    while reset_done & io_pads_bits != io_pads_bits {
        reset_done = cm.read32(resets_reg!(reset_done));
        // The C loop is unbounded; a faulted bus is the only reason to leave it early.
        if cm.check_error() {
            break;
        }
    }

    // Mux the XIP block onto the internal QSPI flash pads.
    for ctrl in [
        gpio_qspi_reg!(sclk_ctrl),
        gpio_qspi_reg!(cs_ctrl),
        gpio_qspi_reg!(sd0_ctrl),
        gpio_qspi_reg!(sd1_ctrl),
        gpio_qspi_reg!(sd2_ctrl),
        gpio_qspi_reg!(sd3_ctrl),
    ] {
        cm.write32(ctrl, 0);
    }
}

/// Put the SSI into plain 8-bit serial mode, where every byte sent gets one back
/// (C `rp_flash_init_spi()`).
///
/// Only the exit-XIP sequence uses this: it leaves the baud rate and clock polarity/phase
/// alone, so user code that set them up keeps them.
fn init_spi(cm: &dyn FlashAccess) {
    cm.write32(ssi_reg!(enable), 0);
    // Clear the sticky errors, which is done by reading them.
    cm.read32(ssi_reg!(sr));
    cm.read32(ssi_reg!(icr));
    cm.write32(ssi_reg!(baud), SSI_BAUD_DIVIDER);
    cm.write32(
        ssi_reg!(ctrl0),
        SSI_CTRL0_FRF_SERIAL | ssi_ctrl0_data_bits(8) | SSI_CTRL0_TMOD_BIDI,
    );
    // The slave is selected while transfers are in progress.
    cm.write32(ssi_reg!(ser), 1);
    cm.write32(ssi_reg!(enable), 1);
}

/// Whether the chip aborted the transfer (it pulls SD1 low when it does)
/// (C `rp_flash_was_aborted()`).
fn was_aborted(cm: &dyn FlashAccess) -> bool {
    cm.read32(gpio_qspi_reg!(sd1_ctrl)) & GPIO_QSPI_SD1_CTRL_INOVER_BITS != 0
}

/// Push `count` bytes through the SSI, keeping the FIFOs fed, and read `count` back
/// (C `rp_flash_put_get()`).
///
/// The first `rx_skip` bytes read are dropped, which is how a command and address that
/// were written just before the call are discarded.
fn put_get(
    cm: &dyn FlashAccess,
    tx: Option<&[u8]>,
    mut rx: Option<&mut [u8]>,
    count: usize,
    mut rx_skip: usize,
) {
    let mut tx_count = 0;
    let mut rx_count = 0;
    while tx_count < count || rx_count < count || rx_skip != 0 {
        // Read the FIFO levels pessimistically: TX first, then RX.
        let tx_level = cm.read32(ssi_reg!(txflr));
        let rx_level = cm.read32(ssi_reg!(rxflr));
        let mut idle = true;
        if tx_count < count && tx_level + rx_level < MAX_IN_FLIGHT {
            cm.write32(ssi_reg!(dr0), u32::from(tx.map_or(0, |buffer| buffer[tx_count])));
            tx_count += 1;
            idle = false;
        }
        if rx_level != 0 {
            let data = cm.read32(ssi_reg!(dr0)) as u8;
            if rx_skip != 0 {
                rx_skip -= 1;
            } else {
                if let Some(buffer) = rx.as_deref_mut() {
                    buffer[rx_count] = data;
                }
                rx_count += 1;
            }
            idle = false;
        }
        // Only check for an abort on idle loops: the status read costs four cycles.
        if idle && was_aborted(cm) {
            break;
        }
        if cm.check_error() {
            break;
        }
    }
    chip_select(cm, GPIO_QSPI_CS_DRIVE_HIGH);
}

/// Take the flash out of QPI/XIP mode so it accepts plain serial commands
/// (C `rp_flash_exit_xip()`).
///
/// The sequence is the one the boot ROM uses: hold the chip select high for 32 clocks
/// with the IO lines pulled down, then low for 32 clocks with them pulled up, then
/// deassert briefly and send `0xff 0xff`, which is what the W25X10CL datasheet asks for.
/// The pull phases are what makes the sequence work on Micron parts too.
fn exit_xip(cm: &dyn FlashAccess) {
    init_spi(cm);

    let padctrl_save = cm.read32(pads_qspi_reg!(gpio_sd0));
    let mut padctrl_tmp = (padctrl_save
        & !(PADS_QSPI_GPIO_SD0_OD_BITS | PADS_QSPI_GPIO_SD0_PUE_BITS | PADS_QSPI_GPIO_SD0_PDE_BITS))
        | PADS_QSPI_GPIO_SD0_OD_BITS
        | PADS_QSPI_GPIO_SD0_PDE_BITS;

    // The two 32-clock sequences, chip select high for the first and low for the second.
    chip_select(cm, GPIO_QSPI_CS_DRIVE_HIGH);
    for _ in 0..2 {
        for pad in [
            pads_qspi_reg!(gpio_sd0),
            pads_qspi_reg!(gpio_sd1),
            pads_qspi_reg!(gpio_sd2),
            pads_qspi_reg!(gpio_sd3),
        ] {
            cm.write32(pad, padctrl_tmp);
        }
        // Give the pulls time to take effect.
        delay_ms(PULL_SETTLE_MS);
        put_get(cm, None, None, 4, 0);
        padctrl_tmp = (padctrl_tmp & !PADS_QSPI_GPIO_SD0_PDE_BITS) | PADS_QSPI_GPIO_SD0_PUE_BITS;
        chip_select(cm, GPIO_QSPI_CS_DRIVE_LOW);
    }

    // Restore the IO/pad controls and send `0xff 0xff`. IO2/IO3 get a pull up as they
    // may be used as ~WP/~HOLD at this point, while serial commands are being issued.
    cm.write32(pads_qspi_reg!(gpio_sd0), padctrl_save);
    cm.write32(pads_qspi_reg!(gpio_sd1), padctrl_save);
    let padctrl_serial = (padctrl_save & !PADS_QSPI_GPIO_SD0_PDE_BITS) | PADS_QSPI_GPIO_SD0_PUE_BITS;
    cm.write32(pads_qspi_reg!(gpio_sd2), padctrl_serial);
    cm.write32(pads_qspi_reg!(gpio_sd3), padctrl_serial);

    chip_select(cm, GPIO_QSPI_CS_DRIVE_LOW);
    put_get(cm, Some(&[0xff, 0xff]), None, 2, 0);

    cm.write32(gpio_qspi_reg!(cs_ctrl), 0);
}

/// Flush the XIP cache, leave it enabled and hand the chip select back to the XIP block
/// (C `rp_flash_flush_cache()`).
fn flush_cache(cm: &dyn FlashAccess) {
    cm.write32(xip_reg!(flush), 1);
    // The read blocks until the flush is done.
    cm.read32(xip_reg!(flush));
    cm.write32(xip_reg!(ctrl) | REG_ACCESS_WRITE_ATOMIC_BITSET, XIP_CTRL_ENABLE);
    chip_select(cm, GPIO_QSPI_CS_DRIVE_NORMAL);
}

/// Put the SSI back into the mode where XIP accesses turn into `0x03` serial reads
/// (C `rp_flash_enter_xip()`).
fn enter_xip(cm: &dyn FlashAccess) {
    cm.write32(ssi_reg!(enable), 0);
    // 32 clocks per frame: instruction and address out, data in.
    cm.write32(
        ssi_reg!(ctrl0),
        SSI_CTRL0_FRF_SERIAL | ssi_ctrl0_data_bits(32) | SSI_CTRL0_TMOD_EEPROM,
    );
    cm.write32(
        ssi_reg!(xip_spi_ctrl0),
        ssi_xip_spi_ctrl0_xip_cmd(XIP_READ_COMMAND)
            | SSI_XIP_SPI_CTRL0_INSTR_LENGTH_8B
            | ssi_xip_spi_ctrl0_address_length(XIP_READ_ADDRESS_BYTES)
            | SSI_XIP_SPI_CTRL0_TRANS_1C1A,
    );
    cm.write32(ssi_reg!(enable), SSI_ENABLE_SSI);
}

/// Save the SSI configuration and put it in the mode the flash layer needs
/// (C `rp_spi_config()`).
fn spi_config(cm: &dyn FlashAccess, state: &mut SsiState) {
    state.enabled = cm.read32(ssi_reg!(enable));
    cm.write32(ssi_reg!(enable), 0);
    state.ctrl0 = cm.read32(ssi_reg!(ctrl0));
    state.ctrl1 = cm.read32(ssi_reg!(ctrl1));
    state.xpi_ctrl0 = cm.read32(ssi_reg!(xip_spi_ctrl0));
    cm.write32(
        ssi_reg!(ctrl0),
        (state.ctrl0 & SSI_CTRL0_MASK) | SSI_CTRL0_FRF_SERIAL | SSI_CTRL0_TMOD_BIDI | ssi_ctrl0_data_bits(8),
    );
    cm.write32(
        ssi_reg!(xip_spi_ctrl0),
        SSI_XIP_SPI_CTRL0_FORMAT_FRF
            | ssi_xip_spi_ctrl0_address_length(0)
            | SSI_XIP_SPI_CTRL0_INSTR_LENGTH_8B
            | ssi_xip_spi_ctrl0_wait_cycles(0),
    );
    cm.write32(ssi_reg!(enable), SSI_ENABLE_SSI);
}

/// Put the SSI back to what [`spi_config`] saved (C `rp_spi_restore()`).
fn spi_restore(cm: &dyn FlashAccess, state: &SsiState) {
    cm.write32(ssi_reg!(enable), 0);
    cm.write32(ssi_reg!(ctrl1), state.ctrl1);
    cm.write32(ssi_reg!(ctrl0), state.ctrl0);
    cm.write32(ssi_reg!(xip_spi_ctrl0), state.xpi_ctrl0);
    cm.write32(ssi_reg!(enable), state.enabled);
}

/// Length of the flash, from the JEDEC id when it decodes, the family maximum otherwise
/// (C `rp_get_flash_length()`).
///
/// The C function also logs the three id bytes at debug level; the size report of
/// `SpiFlash::identify()` is what survives here.
fn flash_length(cm: &dyn FlashAccess) -> usize {
    let id = spi::read_jedec_id(cm, &Ssi);
    // The capacity byte is log2 of the size in bytes; outside the plausible range the
    // chip did not answer properly, so map everything the pads can reach.
    if id.capacity >= 8 && id.capacity <= 34 {
        return match 1u64.checked_shl(u32::from(id.capacity)) {
            // Anything past the probe's address space cannot be mapped anyway.
            Some(length) => usize::try_from(length).unwrap_or(MAX_FLASH),
            None => MAX_FLASH,
        };
    }
    MAX_FLASH
}

/// Discover the flash chip and describe it (C `rp_add_flash()`).
///
/// Runs from the probe, which is where the C driver does it too (its `rp_attach()` is
/// called right after the probe), and leaves the XIP mode exactly as it found it.
pub fn add_flash(cm: &dyn FlashAccess, state: &mut SsiState) -> SpiFlash {
    let needs_config = needs_configuring(cm);
    if needs_config {
        connect_internal(cm);
    }
    exit_xip(cm);
    spi_config(cm, state);

    let flash = SpiFlash::identify(cm, &Ssi, XIP_FLASH_BASE, flash_length(cm));

    spi_restore(cm, state);
    if needs_config {
        flush_cache(cm);
    }
    enter_xip(cm);
    flash
}

/// The SSI, taken over for one flash session.
///
/// Created by [`SsiEngine::engage`] (C `rp_flash_prepare()`), consumed by
/// [`SsiEngine::release`] (C `rp_flash_resume()`). While one exists the XIP controller is out
/// of the way and a core executing from flash would fault, so every operation that touches
/// the chip is a method here: there is no way to erase or program without having taken the
/// controller over first, and no way to release the same saved state twice.
///
/// This is the driver's only mutable state: [`crate::driver::Rp2040Driver`] holds it as an
/// `Option` for the duration of `flash_prepare()`..`flash_done()`.
#[derive(Debug, Clone, Copy)]
pub struct SsiEngine {
    /// The SSI registers to put back in [`Self::release`].
    state: SsiState,
    /// The flash chip answering on the SSI.
    flash: SpiFlash,
}

impl SsiEngine {
    /// Take the SSI over for `flash` (C `rp_flash_prepare()`).
    ///
    /// `load_stub` says whether the write stub has to be uploaded: the driver asks for it once
    /// per session ([`rsbmp::flash::StubState`]) instead of on every prepare, since the manager
    /// prepares per operation change. Erase and mass erase do not use the stub and pass `false`.
    ///
    /// `None` means core 1 is not parked or the stub is not in SRAM, so no erase or write
    /// may follow.
    pub fn engage(cm: &CortexMAccess, flash: SpiFlash, load_stub: bool) -> Option<Self> {
        let mut state = SsiState::default();
        prepare(cm, &mut state, load_stub).then_some(Self { state, flash })
    }

    /// Erase the block holding `addr`.
    ///
    /// The flash manager calls this once per block, so only that block has to go.
    pub fn erase(&mut self, cm: &CortexMAccess, addr: u32) -> bool {
        self.flash.erase(cm, &Ssi, addr)
    }

    /// Program `data` at `dest` with the in-SRAM stub.
    pub fn write(&mut self, cm: &CortexMAccess, dest: u32, data: &[u8]) -> bool {
        write(cm, &self.flash, dest, data)
    }

    /// Erase the whole chip.
    pub fn mass_erase(&mut self, cm: &CortexMAccess) -> bool {
        self.flash.mass_erase(cm, &Ssi)
    }

    /// Give the flash back: put the SSI back, resume XIP and restart the part
    /// (C `rp_flash_resume()`).
    pub fn release(self, cm: &CortexMAccess) {
        resume(cm, &self.state);
    }
}

/// Enter flash mode: park core 1, take the SSI over and, when `load_stub` asks for it, preload
/// the write stub (C `rp_flash_prepare()`).
pub fn prepare(cm: &dyn FlashAccess, state: &mut SsiState, load_stub: bool) -> bool {
    // Core 1 must not run from flash (or do anything else) while it is reprogrammed.
    cm.write32(psm_reg!(frce_off) | REG_ACCESS_WRITE_ATOMIC_BITSET, PSM_FRCE_OFF_PROC1_BITS);
    connect_internal(cm);
    exit_xip(cm);
    spi_config(cm, state);
    if load_stub {
        cm.mem_write(SRAM_BASE, &crate::flashstub_write::RP2040_WRITE_BIN);
    }
    !cm.check_error()
}

/// Leave flash mode: put the SSI back, resume XIP and restart the part
/// (C `rp_flash_resume()`).
///
/// Swindle calls this from the driver's flash-level `done` hook, which the flash manager
/// runs once per GDB flash request (erase, then write), so the part is restarted between
/// those two operations as well as at the end of the request. That matches what the C
/// driver ends up doing: its `rp_flash_resume()` is the target's *exit flash mode* hook,
/// which BMP also runs between the erase and the write of a `load`.
pub fn resume(cm: &dyn FlashAccess, state: &SsiState) {
    spi_restore(cm, state);
    flush_cache(cm);
    enter_xip(cm);
    // Release core 1 now that the flash is back in XIP mode.
    cm.write32(psm_reg!(frce_off) | REG_ACCESS_WRITE_ATOMIC_BITCLR, PSM_FRCE_OFF_PROC1_BITS);
    // Restart the part, as C does: the target must not be left running old code after the
    // flash was reprogrammed.
    cm.reset();
}

/// Program `data` at `dest` with the stub, which does the page writes at core speed
/// (C `rp_flash_write()`).
// The write stub image sits at the base of SRAM and must stay below the CRC stub's slot.
const _: () = assert!(SRAM_BASE + crate::flashstub_write::RP2040_WRITE_BIN_LEN as u32 <= CRC_STUB_ADDR);
pub fn write(cm: &CortexMAccess, flash: &SpiFlash, dest: u32, data: &[u8]) -> bool {
    // Load the next block of data, then run the stub next to it. The stub publishes the
    // common stub exit code: `bkpt #1` once the whole chunk is in the chip, `bkpt #0` when
    // the flash refused a write enable.
    cm.mem_write(STUB_BUFFER_BASE, data);
    if cm.check_error() {
        return false;
    }
    cm.run_stub(
        SRAM_BASE,
        dest - flash.start,
        STUB_BUFFER_BASE,
        data.len() as u32,
        flash.page_size as u32,
    )
}


