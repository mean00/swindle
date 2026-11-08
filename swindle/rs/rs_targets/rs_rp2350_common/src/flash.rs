//! The RP2350 flash controller: the QMI taken into direct mode, and the SPI transport.
//!
//! Rust port of the flash half of `blackmagic/src/target/rp2350.c`: `rp2350_add_flash()`,
//! `rp2350_flash_prepare()`/`_resume()`, `rp2350_spi_prepare()`/`_resume()`, the FIFO
//! draining, the exit-XIP sequence and the three QMI access helpers that make up the
//! transport the shared flash layer ([`rs_rp_common::spi`]) drives.
//!
//! Unlike the RP2040 there is no SRAM stub: the QMI can program a page out of the
//! transmit FIFO, so the shared layer's own page loop is what writes the chip (the C
//! driver passes `bmp_spi_flash_write()` to the flash manager for the same reason).
//! Programming through the QMI means the XIP cache has to be told to drop the lines
//! being touched, which [`Qmi::write`] does as it goes.

use crate::registers::*;
use rs_rp_common::access::FlashAccess;
use rs_rp_common::spi::{self, SpiFlash, SpiTransport};
use rsbmp::target::delay_ms;

/// Settling time for the pad pulls during the exit-XIP sequence, in milliseconds
/// (C `platform_delay(10U)`).
const PULL_SETTLE_MS: u32 = 10;

/// Bytes per XIP cache line: a page program has to drop the line holding every 8 bytes it
/// writes (C `(i & 7U) == 6U` in `rp2350_spi_write()`).
const CACHE_LINE_BYTES: usize = 8;

/// Opcode of the "exit QPI mode" command (C `0xf5`).
const EXIT_QPI_OPCODE: u8 = 0xf5;
/// The all-ones byte that closes the exit sequence (C `0xff`).
const ALL_ONES_BYTE: u32 = 0xff;
/// The 16-bit all-ones pattern of the clock-only phases (C `0xffff`).
const ALL_ONES_HALFWORD: u32 = 0xffff;

/// The `0xf5` exit-QPI command, sent as one quad byte (C `rp2350_flash_exit_xip()`).
const EXIT_QPI_COMMAND: u32 = QMI_DIRECT_TX_MODE_QUAD
    | QMI_DIRECT_TX_DATA_8BIT
    | QMI_DIRECT_TX_OUTPUT_ENABLE
    | QMI_DIRECT_TX_NOPUSH_RX
    | EXIT_QPI_OPCODE as u32;
/// 16 clocks of ones (C `..._MODE_SINGLE | ..._DATA_16BIT | ..._NOPUSH_RX | 0xffff`).
const ALL_ONES_16_CLOCKS: u32 =
    QMI_DIRECT_TX_MODE_SINGLE | QMI_DIRECT_TX_DATA_16BIT | QMI_DIRECT_TX_NOPUSH_RX | ALL_ONES_HALFWORD;
/// The closing `0xff`, sent as one quad byte (C `..._MODE_QUAD | ..._DATA_8BIT | ... | 0xff`).
const ALL_ONES_QUAD_BYTE: u32 = QMI_DIRECT_TX_MODE_QUAD
    | QMI_DIRECT_TX_DATA_8BIT
    | QMI_DIRECT_TX_OUTPUT_ENABLE
    | QMI_DIRECT_TX_NOPUSH_RX
    | ALL_ONES_BYTE;

/// The QMI driven in direct mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct Qmi;

impl SpiTransport for Qmi {
    fn read(&self, m: &dyn FlashAccess, command: u16, address: u32, buffer: &mut [u8]) {
        setup_xfer(m, command, address);
        for byte in buffer.iter_mut() {
            // A write to the transmit register is what clocks a byte in.
            m.write32(
                qmi_reg!(direct_tx),
                QMI_DIRECT_TX_MODE_SINGLE | QMI_DIRECT_TX_DATA_8BIT,
            );
            *byte = m.read32(qmi_reg!(direct_rx)) as u8;
        }
        deselect(m);
    }

    fn write(&self, m: &dyn FlashAccess, command: u16, address: u32, buffer: &[u8]) {
        setup_xfer(m, command, address);
        // Two bytes per transmit register write.
        let mut offset = 0;
        while offset < buffer.len() {
            let low = u32::from(buffer[offset]);
            // A trailing odd byte is sent as a halfword with a zero in its high half; a
            // page is always a power of two, so this only matters for odd commands.
            let high = u32::from(buffer.get(offset + 1).copied().unwrap_or(0));
            m.write32(
                qmi_reg!(direct_tx),
                QMI_DIRECT_TX_MODE_SINGLE
                    | QMI_DIRECT_TX_DATA_16BIT
                    | QMI_DIRECT_TX_NOPUSH_RX
                    | low
                    | (high << 8),
            );
            // Every complete 8 bytes when page programming, drop the cache line holding
            // them: the XIP cache would otherwise keep serving the stale contents.
            if command == spi::CMD_PAGE_PROGRAM && offset % CACHE_LINE_BYTES == CACHE_LINE_BYTES - 2 {
                let line = XIP_CACHE_BASE + XIP_CACHE_INVALIDATE_BY_ADDRESS + address + (offset as u32 & !(CACHE_LINE_BYTES as u32 - 1));
                m.write32(line, 0);
            }
            offset += 2;
        }
        wait_not_busy(m);
        deselect(m);
    }

    fn run_command(&self, m: &dyn FlashAccess, command: u16, address: u32) {
        setup_xfer(m, command, address);
        wait_not_busy(m);
        deselect(m);
    }
}

/// Pull chip select 0 high again, which is what makes the chip execute the command
/// (C `write(QMI_DIRECT_CSR | BITCLR, ASSERT_CS0N)`).
fn deselect(m: &dyn FlashAccess) {
    m.write32(
        qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITCLR,
        QMI_DIRECT_CSR_ASSERT_CS0N,
    );
}

/// Wait until the QMI has finished clocking the queued frames out.
fn wait_not_busy(m: &dyn FlashAccess) {
    let mut timeout = 50_000usize;
    while m.read32(qmi_reg!(direct_csr)) & QMI_DIRECT_CSR_BUSY != 0 && timeout > 0 {
        if m.check_error() {
            break;
        }
        timeout -= 1;
    }
}

/// Start a transaction: assert chip select 0 and send the command, address and dummy
/// bytes (C `rp2350_spi_setup_xfer()`).
fn setup_xfer(m: &dyn FlashAccess, command: u16, address: u32) {
    m.write32(
        qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITSET,
        QMI_DIRECT_CSR_ASSERT_CS0N,
    );

    let opcode = u32::from(command & spi::OPCODE_MASK);
    m.write32(
        qmi_reg!(direct_tx),
        QMI_DIRECT_TX_MODE_SINGLE | QMI_DIRECT_TX_DATA_8BIT | QMI_DIRECT_TX_NOPUSH_RX | opcode,
    );

    if command & spi::OPCODE_MODE_MASK == spi::OPCODE_3B_ADDR {
        for shift in [16, 8, 0] {
            m.write32(
                qmi_reg!(direct_tx),
                QMI_DIRECT_TX_MODE_SINGLE
                    | QMI_DIRECT_TX_DATA_8BIT
                    | QMI_DIRECT_TX_NOPUSH_RX
                    | ((address >> shift) & 0xff),
            );
        }
    }

    let dummy_bytes = usize::from((command & spi::DUMMY_MASK) >> spi::DUMMY_SHIFT);
    for _ in 0..dummy_bytes {
        m.write32(
            qmi_reg!(direct_tx),
            QMI_DIRECT_TX_MODE_SINGLE | QMI_DIRECT_TX_DATA_8BIT | QMI_DIRECT_TX_NOPUSH_RX,
        );
    }
}


/// Empty both QMI FIFOs (C `rp2350_spi_drain_fifos()`).
fn drain_fifos(m: &dyn FlashAccess, mut status: u32) {
    let mut timeout = 10_000usize;
    while (!(status & (QMI_DIRECT_CSR_RXEMPTY | QMI_DIRECT_CSR_TXEMPTY) != 0)
        || status & QMI_DIRECT_CSR_BUSY != 0)
        && timeout > 0
    {
        if status == 0xffff_ffff || m.check_error() {
            break;
        }
        if status & QMI_DIRECT_CSR_RXEMPTY == 0 {
            m.read32(qmi_reg!(direct_rx));
        }
        status = m.read32(qmi_reg!(direct_csr));
        timeout -= 1;
    }
}

/// Take the chip out of QPI mode and into plain SPI, through direct mode
/// (C `rp2350_flash_exit_xip()`).
fn exit_xip(m: &dyn FlashAccess) {
    // Bring the QMI into a known state and let it drive chip select 0 itself.
    m.write32(
        qmi_reg!(direct_csr),
        QMI_DIRECT_CSR_CLKDIV_DEFAULT | QMI_DIRECT_CSR_AUTO_CS0N,
    );
    // Enter direct mode, empty the FIFOs, leave it again.
    m.write32(
        qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITSET,
        QMI_DIRECT_CSR_DIRECT_ENABLE,
    );
    drain_fifos(m, m.read32(qmi_reg!(direct_csr)));
    m.write32(
        qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITCLR,
        QMI_DIRECT_CSR_DIRECT_ENABLE,
    );

    // Two rounds of 32 clocks, first with the lines pulled down and then pulled up, to
    // get a chip stuck in QPI mode (or deep power-down) back to its senses.
    let mut padctrl = (m.read32(pads_qspi_reg!(sd0)) & !PADS_QSPI_GPIO_PULL_UP_ENABLE)
        | PADS_QSPI_GPIO_OUTPUT_DISABLE
        | PADS_QSPI_GPIO_PULL_DOWN_ENABLE;
    for _ in 0..2 {
        for pad in [
            pads_qspi_reg!(sd0),
            pads_qspi_reg!(sd1),
            pads_qspi_reg!(sd2),
            pads_qspi_reg!(sd3),
        ] {
            m.write32(pad, padctrl);
        }
        // Give the pulls time to take effect.
        delay_ms(PULL_SETTLE_MS);

        m.write32(
            qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITSET,
            QMI_DIRECT_CSR_DIRECT_ENABLE | QMI_DIRECT_CSR_ASSERT_CS0N,
        );
        for _ in 0..2 {
            m.write32(
                qmi_reg!(direct_tx),
                QMI_DIRECT_TX_MODE_SINGLE
                    | QMI_DIRECT_TX_DATA_16BIT
                    | QMI_DIRECT_TX_NOPUSH_RX
                    | ALL_ONES_HALFWORD,
            );
        }
        wait_not_busy(m);
        m.write32(
            qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITCLR,
            QMI_DIRECT_CSR_DIRECT_ENABLE | QMI_DIRECT_CSR_ASSERT_CS0N,
        );

        padctrl ^= PADS_QSPI_GPIO_PULL_UP_ENABLE | PADS_QSPI_GPIO_PULL_DOWN_ENABLE;
    }

    // Drive the lines again, with SD2 and SD3 pulled up in case they are ~WP/~HOLD.
    padctrl ^= PADS_QSPI_GPIO_OUTPUT_DISABLE;
    m.write32(pads_qspi_reg!(sd0), padctrl);
    m.write32(pads_qspi_reg!(sd1), padctrl);
    padctrl ^= PADS_QSPI_GPIO_PULL_UP_ENABLE | PADS_QSPI_GPIO_PULL_DOWN_ENABLE;
    m.write32(pads_qspi_reg!(sd2), padctrl);
    m.write32(pads_qspi_reg!(sd3), padctrl);

    // The `0xf5` "exit QPI" command, then all ones: once as 16 clocks of 1s and once as a
    // single quad byte. Each phase asserts chip select, clocks the frame and releases it.
    for command in [
        EXIT_QPI_COMMAND,
        ALL_ONES_16_CLOCKS,
        ALL_ONES_QUAD_BYTE,
    ] {
        m.write32(
            qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITSET,
            QMI_DIRECT_CSR_DIRECT_ENABLE | QMI_DIRECT_CSR_ASSERT_CS0N,
        );
        m.write32(qmi_reg!(direct_tx), command);
        wait_not_busy(m);
        m.write32(
            qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITCLR,
            QMI_DIRECT_CSR_DIRECT_ENABLE | QMI_DIRECT_CSR_ASSERT_CS0N,
        );
    }
}

/// Take the QMI into direct access mode, configuring the pads when the part comes out of
/// reset first (C `rp2350_spi_prepare()`).
///
/// Returns whether this call is what switched direct mode on, which is what
/// [`add_flash`] uses to know whether the mode has to be handed back afterwards.
pub fn spi_prepare(m: &dyn FlashAccess) -> bool {
    // Is the QMI muxed out to the pads at all?
    if m.read32(gpio_qspi_reg!(sclk_ctrl)) & GPIO_QSPI_CTRL_FUNCSEL_MASK == GPIO_QSPI_CTRL_FUNCSEL_NONE {
        // Reset the GPIO and pads controllers to a known state.
        let io_pads = RESETS_RESET_IO_QSPI | RESETS_RESET_PADS_QSPI;
        m.write32(resets_reg!(reset) | REG_ACCESS_WRITE_ATOMIC_BITSET, io_pads);
        m.write32(resets_reg!(reset) | REG_ACCESS_WRITE_ATOMIC_BITCLR, io_pads);
        let mut timeout = 10_000usize;
        while m.read32(resets_reg!(reset_done)) & io_pads != io_pads && timeout > 0 {
            // The C loop is unbounded; a faulted bus is the only reason to leave it.
            if m.check_error() {
                break;
            }
            timeout -= 1;
        }

        // Hand the GPIOs to the QMI.
        for ctrl in [
            gpio_qspi_reg!(sclk_ctrl),
            gpio_qspi_reg!(cs_ctrl),
            gpio_qspi_reg!(sd0_ctrl),
            gpio_qspi_reg!(sd1_ctrl),
            gpio_qspi_reg!(sd2_ctrl),
            gpio_qspi_reg!(sd3_ctrl),
        ] {
            m.write32(ctrl, 0);
        }

        // Un-isolate the pads so the QMI can see the outside world.
        for pad in [
            pads_qspi_reg!(sclk),
            pads_qspi_reg!(cs),
            pads_qspi_reg!(sd0),
            pads_qspi_reg!(sd1),
            pads_qspi_reg!(sd2),
            pads_qspi_reg!(sd3),
        ] {
            m.write32(pad | REG_ACCESS_WRITE_ATOMIC_BITCLR, PADS_QSPI_GPIO_ISOLATE);
        }

        exit_xip(m);
    }

    // Clear the state that inhibits the flash, if the chip is in it.
    if m.read32(gpio_qspi_reg!(sd1_ctrl)) & GPIO_QSPI_CTRL_INOVER_MASK != GPIO_QSPI_CTRL_INOVER_NONE {
        m.write32(gpio_qspi_reg!(sd1_ctrl), 0);
    }

    let state = m.read32(qmi_reg!(direct_csr));
    if state & QMI_DIRECT_CSR_DIRECT_ENABLE == 0 {
        // Turn direct mode on and wait for the transaction in progress to stop.
        m.write32(
            qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITSET,
            QMI_DIRECT_CSR_DIRECT_ENABLE,
        );
        let mut timeout = 10_000usize;
        while m.read32(qmi_reg!(direct_csr)) & QMI_DIRECT_CSR_BUSY != 0 && timeout > 0 {
            if m.check_error() {
                break;
            }
            timeout -= 1;
        }
    } else {
        // Already in direct mode: empty the FIFOs and drop both chip selects.
        drain_fifos(m, state);
        m.write32(
            qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITCLR,
            QMI_DIRECT_CSR_ASSERT_CS0N | QMI_DIRECT_CSR_ASSERT_CS1N,
        );
    }

    state & QMI_DIRECT_CSR_DIRECT_ENABLE == 0
}

/// Turn direct access mode back off, which re-maps the flash at the XIP window again
/// (C `rp2350_spi_resume()`).
pub fn spi_resume(m: &dyn FlashAccess) {
    m.write32(
        qmi_reg!(direct_csr) | REG_ACCESS_WRITE_ATOMIC_BITCLR,
        QMI_DIRECT_CSR_DIRECT_ENABLE,
    );
}

/// Invalidate XIP cache via the XIP_CTRL FLUSH hardware register (0x400C0004).
pub fn flush_cache(m: &dyn FlashAccess, _addr: u32, _len: usize) {
    m.write32(0x400c_0004, 1);
    let mut timeout = 10_000usize;
    while m.read32(0x400c_0008) & 1 == 0 && timeout > 0 {
        if m.check_error() {
            break;
        }
        timeout -= 1;
    }
}

/// Discover the flash chip and describe it (C `rp2350_add_flash()`).
///
/// `None` when no chip answered the JEDEC id read, in which case only the SRAM ends up
/// in the memory map.
pub fn add_flash(m: &dyn FlashAccess) -> Option<SpiFlash> {
    let mode_switched = spi_prepare(m);
    let id = spi::read_jedec_id(m, &Qmi);

    let flash = if id.is_present() {
        // The capacity byte is log2 of the size in bytes, clipped to what the XIP block
        // decodes.
        let capacity = 1usize << id.capacity.min(31);
        Some(SpiFlash::identify(
            m,
            &Qmi,
            XIP_FLASH_BASE,
            capacity.min(XIP_FLASH_SIZE),
        ))
    } else {
        None
    };

    if mode_switched {
        spi_resume(m);
    }
    flash
}

/// Enter flash mode: hand the QMI over to direct access (C `rp2350_flash_prepare()`).
///
/// `true` when this is what turned direct mode on, i.e. when
/// [`spi_resume`] has to turn it off again (C `rp2350_spi_prepare()`'s return value).
pub fn prepare(m: &dyn FlashAccess) -> bool {
    spi_prepare(m)
}

/// Leave flash mode: give the flash back to the XIP block
/// (C `rp2350_flash_resume()`).
pub fn resume(m: &dyn FlashAccess) {
    spi_resume(m);
}

/// The QMI, taken over for one flash session.
///
/// Created by [`QmiEngine::engage`] (C `rp2350_flash_prepare()`), consumed by
/// [`QmiEngine::release`] (C `rp2350_flash_resume()`). While one exists the QMI is in direct
/// mode and a core executing from flash faults, so every operation that touches the chip is a
/// method here: nothing can erase or program without a taken-over controller, and the mode is
/// handed back exactly once.
///
/// Both RP2350 drivers hold one as an `Option` for the duration of
/// `flash_prepare()`..`flash_done()`: the Cortex-M33 half in `rs_rp2350_arm`, the Hazard3 half
/// in `rs_rp2350_riscv`.
#[derive(Debug, Clone, Copy)]
pub struct QmiEngine {
    /// The flash chip answering on the QMI.
    flash: SpiFlash,
}

impl QmiEngine {
    /// Take the QMI over for `flash` (C `rp2350_flash_prepare()`).
    pub fn engage(m: &dyn FlashAccess, flash: SpiFlash) -> Self {
        // `prepare()` also reports whether *this* call switched direct mode on; `resume()`
        // puts the mode back either way, so the flag has no use here.
        prepare(m);
        Self { flash }
    }

    /// Erase the block holding `addr`.
    ///
    /// The flash manager calls this once per block, so only that block has to go.
    pub fn erase(&mut self, m: &dyn FlashAccess, addr: u32) -> bool {
        self.flash.erase(m, &Qmi, addr)
    }

    /// Program `data` at `dest`.
    pub fn write(&mut self, m: &dyn FlashAccess, dest: u32, data: &[u8]) -> bool {
        self.flash.write(m, &Qmi, dest, data)
    }

    /// Erase the whole chip.
    pub fn mass_erase(&mut self, m: &dyn FlashAccess) -> bool {
        self.flash.mass_erase(m, &Qmi)
    }

    /// Give the flash back to the XIP block and restart the part
    /// (C `rp2350_flash_resume()`).
    pub fn release(self, m: &dyn FlashAccess) {
        resume(m);
    }
}
