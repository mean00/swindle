//! RISC-V memory access method benchmark harness.
//!
//! Benchmarks available memory access methods:
//! - Sysbus (hardware auto-increment)
//! - Abstract (word-wise abstract commands)
//! - AbstAuto (streaming abstract commands with autoexecdata_0)
//! - Progbuf (program buffer execution)
//!
//! Round-trips 1 word and 64 words through SRAM (0x20000000), reporting
//! DMI transaction counts and verification status. The hart is halted for the
//! duration of the run, and the 256-byte scratch window is saved before it and
//! written back afterwards, so benchmarking a target that is running a live image
//! does not destroy it.

use super::dm_regs::RV_DM_PROGBUF_MIN_WORDS;
use super::dmi::DmiTransport;
use super::hart::{MemoryAccessMethod, RiscvHart};
use super::mem::RiscvMem;
use crate::target::driver::HaltReason;

/// Start of the scratch window: the lowest SRAM address, the only one every supported part has
/// (2 KiB on a CH32V003, 64 KiB on a CH32V303). A fixed address cannot be free RAM on a target
/// that is running a live image, which is why the window is snapshotted and restored around the run.
pub const RISCV_BENCH_SCRATCH_ADDR: u32 = 0x20000000;
pub const RISCV_BENCH_TEST_VALUE: u32 = 0xa5a55a5a;
pub const RISCV_BENCH_WORDS: usize = 64;

unsafe extern "C" {
    fn bmp_mem_counts_reset_c();
    fn bmp_mem_counts_c(swd: *mut u32, rv: *mut u32);
}

fn dmi_tx_reset() {
    unsafe { bmp_mem_counts_reset_c() };
}

fn dmi_tx_count() -> u32 {
    let mut swd = 0u32;
    let mut rv = 0u32;
    unsafe { bmp_mem_counts_c(&mut swd, &mut rv) };
    rv
}

fn init_scratch() -> [u8; RISCV_BENCH_WORDS * 4] {
    let mut scratch = [0u8; RISCV_BENCH_WORDS * 4];
    for i in 0..RISCV_BENCH_WORDS {
        let val = RISCV_BENCH_TEST_VALUE.wrapping_add(i as u32);
        scratch[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
    }
    scratch
}

/// What [`halt_for_run`] managed to do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HaltOutcome {
    /// The hart was already stopped: the run leaves it that way.
    AlreadyHalted,
    /// This call stopped the hart and therefore owes it a resume.
    Stopped,
    /// The halt request was never acknowledged: memory must not be touched.
    Failed,
}

/// Stop the hart for the duration of the run.
///
/// The window is dirty while the benchmark writes its pattern: an interrupt taken in the middle of
/// it would find a half-written `.data`, which is how a benchmark on a live image used to end up in
/// `deadEnd`. Nothing may execute between the snapshot and the restore, so the hart is halted first.
///
/// A hart that was already halted stays halted, and one whose state cannot be read is left alone.
/// When the halt request is not acknowledged the caller has to give up rather than write the
/// pattern into a running target - that is the failure this enum exists to report.
fn halt_for_run(hart: &mut RiscvHart) -> HaltOutcome {
    let (reason, _) = hart.halt_poll();
    if reason != HaltReason::Running {
        return HaltOutcome::AlreadyHalted;
    }
    match hart.halt_request() {
        Ok(()) => HaltOutcome::Stopped,
        Err(_) => HaltOutcome::Failed,
    }
}

/// Read the scratch window before the benchmark touches it.
///
/// `None` when the window is not readable, in which case there is nothing worth restoring.
fn snapshot_scratch(hart: &mut RiscvHart) -> Option<[u8; RISCV_BENCH_WORDS * 4]> {
    let mut saved = [0u8; RISCV_BENCH_WORDS * 4];
    RiscvMem::read(hart, RISCV_BENCH_SCRATCH_ADDR, &mut saved).ok()?;
    Some(saved)
}

#[derive(Clone, Copy)]
pub struct BenchResult {
    pub tx_single: u32,
    pub tx_bulk: u32,
    pub ok_single: bool,
    pub ok_bulk: bool,
}

/// Run full benchmark over all supported access methods and report to GDB.
#[inline(never)]
pub fn run_benchmark(hart: &mut RiscvHart) -> bool {
    crate::glue::gdb_out_rs("Running RISC-V memory benchmark at 0x20000000\n");
    let halt = halt_for_run(hart);
    if halt == HaltOutcome::Failed {
        crate::gdb_println!("riscv_benchmark: the hart would not halt, aborted before touching memory");
        return false;
    }
    let resume_at_end = halt == HaltOutcome::Stopped;
    let saved = snapshot_scratch(hart);
    let scratch = init_scratch();

    // 1. Sysbus
    if hart.has_sysbus {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::sysbus_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::sysbus_read(d, a, dst).is_ok(),
        );
        print_result("Sysbus", res);
    } else {
        print_unsupported("Sysbus");
    }

    // 2. Abstract
    {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::abstract_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::abstract_read(d, a, dst).is_ok(),
        );
        print_result("Abstract", res);
    }

    // 3. AbstAuto
    if hart.has_abstauto {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::abstract_auto_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::abstract_auto_read(d, a, dst).is_ok(),
        );
        print_result("AbstAuto", res);
    } else {
        print_unsupported("AbstAuto");
    }

    // 4. Progbuf
    if hart.progbuf_size >= RV_DM_PROGBUF_MIN_WORDS {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::progbuf_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::progbuf_read(d, a, dst).is_ok(),
        );
        print_result("Progbuf", res);
    } else {
        print_unsupported("Progbuf");
    }

    // Give the window back: the target may be running an image that lives there.
    if let Some(saved) = saved {
        let _ = RiscvMem::write(hart, RISCV_BENCH_SCRATCH_ADDR, &saved);
    }
    if resume_at_end {
        let _ = hart.halt_resume(false);
    }

    true
}

/// Name of a memory access method, for the GDB-visible report and the benchmarks.
pub fn method_name(method: MemoryAccessMethod) -> &'static str {
    match method {
        MemoryAccessMethod::Sysbus => "Sysbus",
        MemoryAccessMethod::AbstAuto => "AbstAuto",
        MemoryAccessMethod::Abstract => "Abstract",
        MemoryAccessMethod::Progbuf => "Progbuf",
    }
}

/// Find out what this Debug Module can do to reach memory, and keep the fastest
/// accessor that verifies.
///
/// This *is* the capability detection: there is no register that says "the
/// abstract memory command exists". A QingKe V2 DM (the CH32V003 among others)
/// answers cmdtype 2 with cmderr "not supported" and only offers register access
/// plus its program buffer instead (WCH *QingKeV2 Microprocessor Debug Manual*
/// §3, "V2 microprocessor debug module abstract command only supports the
/// register access mode"), so the candidates are tried in order of what they
/// cost and the first one that round-trips a block wins:
///
/// - Sysbus, when `SBCS` reported a bus width: hardware auto-increment.
/// - Abstract (cmdtype 2): one command per element, and the baseline.
/// - AbstAuto: the same command with the autoexec stream armed.
/// - Progbuf: the halted core executes a load/store per element.
///
/// It runs inside [`RiscvHart::probe`](super::hart::RiscvHart::probe), i.e.
/// *before* any driver probe: the per-family probes read their signature out of
/// the ESIG window first thing, so the accessor has to be known before the first
/// memory read - that ordering is why a CH32V003 could not be identified while
/// this only ran after the probe.
#[inline(never)]
pub fn auto_select_fastest_method(hart: &mut RiscvHart) {
    let saved = snapshot_scratch(hart);
    let scratch = init_scratch();
    // Nothing verified: keep the baseline (a target with no working accessor still
    // reports Abstract, and the access itself reports the failure).
    let mut best_method = MemoryAccessMethod::Abstract;
    let mut best_tx = u32::MAX;

    // 1. Sysbus - only if the hardware reported a bus width.
    if hart.has_sysbus {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::sysbus_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::sysbus_read(d, a, dst).is_ok(),
        );
        if res.ok_bulk {
            best_tx = res.tx_bulk;
            best_method = MemoryAccessMethod::Sysbus;
        }
    }

    // 2. Abstract (cmdtype 2) - the baseline, and the question the capability
    //    detection is really asking: a DM without the command fails here.
    let abstract_ok = {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::abstract_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::abstract_read(d, a, dst).is_ok(),
        );
        if res.ok_bulk && res.tx_bulk < best_tx {
            best_tx = res.tx_bulk;
            best_method = MemoryAccessMethod::Abstract;
        }
        res.ok_bulk
    };

    // 3. AbstAuto - the abstract command with the autoexec stream armed, so it can
    //    only work where the plain command does.
    if abstract_ok && hart.has_abstauto {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::abstract_auto_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::abstract_auto_read(d, a, dst).is_ok(),
        );
        if res.ok_bulk && res.tx_bulk < best_tx {
            best_tx = res.tx_bulk;
            best_method = MemoryAccessMethod::AbstAuto;
        }
    }

    // 4. Progbuf - the only accessor a part without the abstract memory command
    //    has, and slower than it elsewhere, so it only wins where the others lost.
    if !abstract_ok && hart.progbuf_size >= RV_DM_PROGBUF_MIN_WORDS {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::progbuf_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::progbuf_read(d, a, dst).is_ok(),
        );
        if res.ok_bulk && res.tx_bulk < best_tx {
            best_method = MemoryAccessMethod::Progbuf;
        }
    }

    hart.active_method = best_method;

    // Restore scratch window so probing does not corrupt target RAM
    if let Some(saved) = saved {
        let _ = RiscvMem::write(hart, RISCV_BENCH_SCRATCH_ADDR, &saved);
    }
}

/// Report the accessor a scan settled on - the line GDB shows after attaching.
#[inline(never)]
pub fn report_selected_method(hart: &RiscvHart) {
    crate::glue::gdb_out_rs("Selected memory access method: ");
    crate::glue::gdb_out_rs(method_name(hart.active_method));
    crate::glue::gdb_out_rs("\n");
}

/// Run benchmark against the generic active memory dispatch path.
#[inline(never)]
pub fn run_benchmark2(hart: &mut RiscvHart) -> bool {
    let halt = halt_for_run(hart);
    if halt == HaltOutcome::Failed {
        crate::gdb_println!("riscv_benchmark2: the hart would not halt, aborted before touching memory");
        return false;
    }
    let resume_at_end = halt == HaltOutcome::Stopped;
    let saved = snapshot_scratch(hart);
    let scratch = init_scratch();

    let active_name = method_name(hart.active_method);

    dmi_tx_reset();
    let ok_w1 = RiscvMem::write(hart, RISCV_BENCH_SCRATCH_ADDR, &scratch[..4]).is_ok();
    let mut r1 = [0u8; 4];
    let ok_r1 = RiscvMem::read(hart, RISCV_BENCH_SCRATCH_ADDR, &mut r1).is_ok();
    let tx_single = dmi_tx_count();
    let ok_single = ok_w1 && ok_r1 && r1 == scratch[..4];

    dmi_tx_reset();
    let ok_w64 = RiscvMem::write(hart, RISCV_BENCH_SCRATCH_ADDR, &scratch).is_ok();
    let mut r64 = [0u8; RISCV_BENCH_WORDS * 4];
    let ok_r64 = RiscvMem::read(hart, RISCV_BENCH_SCRATCH_ADDR, &mut r64).is_ok();
    let tx_bulk = dmi_tx_count();
    let ok_bulk = ok_w64 && ok_r64 && r64 == scratch;

    let res = BenchResult {
        tx_single,
        tx_bulk,
        ok_single,
        ok_bulk,
    };

    let status = if res.ok_single && res.ok_bulk { "OK" } else { "FAILED" };
    crate::gdb_println!("riscv_benchmark2: ", active_name, " 1-word: ", res.tx_single, " DMI, 64-word: ", res.tx_bulk, " DMI ", status);

    // Give the window back: the target may be running an image that lives there.
    if let Some(saved) = saved {
        let _ = RiscvMem::write(hart, RISCV_BENCH_SCRATCH_ADDR, &saved);
    }
    if resume_at_end {
        let _ = hart.halt_resume(false);
    }

    res.ok_single && res.ok_bulk
}

#[inline(never)]
fn bench_method<W, R>(
    dmi: &mut dyn DmiTransport,
    scratch: &[u8],
    mut write_fn: W,
    mut read_fn: R,
) -> BenchResult
where
    W: FnMut(&mut dyn DmiTransport, u32, &[u8]) -> bool,
    R: FnMut(&mut dyn DmiTransport, u32, &mut [u8]) -> bool,
{
    // 1-word test
    dmi_tx_reset();
    let ok_w1 = write_fn(dmi, RISCV_BENCH_SCRATCH_ADDR, &scratch[..4]);
    let mut r1 = [0u8; 4];
    let ok_r1 = read_fn(dmi, RISCV_BENCH_SCRATCH_ADDR, &mut r1);
    let tx_single = dmi_tx_count();
    let ok_single = ok_w1 && ok_r1 && r1 == scratch[..4];

    // 64-word test
    dmi_tx_reset();
    let ok_w64 = write_fn(dmi, RISCV_BENCH_SCRATCH_ADDR, scratch);
    let mut r64 = [0u8; RISCV_BENCH_WORDS * 4];
    let ok_r64 = read_fn(dmi, RISCV_BENCH_SCRATCH_ADDR, &mut r64);
    let tx_bulk = dmi_tx_count();
    let ok_bulk = ok_w64 && ok_r64 && r64 == scratch;

    BenchResult {
        tx_single,
        tx_bulk,
        ok_single,
        ok_bulk,
    }
}

#[inline(never)]
fn print_result(name: &str, res: BenchResult) {
    if res.ok_single && res.ok_bulk {
        crate::gdb_println!(name, ": OK (1-word: ", res.tx_single, " DMI, 64-word: ", res.tx_bulk, " DMI)");
    } else {
        crate::gdb_println!(name, ": FAILED");
    }
}

#[inline(never)]
fn print_unsupported(name: &str) {
    crate::gdb_println!(name, ": UNSUPPORTED");
}
