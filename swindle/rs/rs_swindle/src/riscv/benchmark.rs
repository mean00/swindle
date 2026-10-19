//! RISC-V memory access method benchmark harness.
//!
//! Benchmarks available memory access methods:
//! - Sysbus (hardware auto-increment)
//! - Abstract (word-wise abstract commands)
//! - AbstAuto (streaming abstract commands with autoexecdata_0)
//! - Progbuf (program buffer execution)
//!
//! Round-trips 1 word and 64 words through SRAM (0x20000000), reporting
//! DMI transaction counts and verification status.

use super::dmi::DmiTransport;
use super::hart::{MemoryAccessMethod, RiscvHart};
use super::mem::RiscvMem;

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
    print_unsupported("Progbuf");

    true
}

/// Automatically benchmark supported memory access methods and select the fastest one.
///
/// Runs quietly without printing to GDB output, and updates `hart.active_method`.
#[inline(never)]
pub fn auto_select_fastest_method(hart: &mut RiscvHart) {
    let scratch = init_scratch();
    let mut best_method = MemoryAccessMethod::Abstract;
    let mut best_tx = u32::MAX;

    // 1. Abstract (baseline: always supported)
    {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::abstract_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::abstract_read(d, a, dst).is_ok(),
        );
        if res.ok_bulk {
            best_tx = res.tx_bulk;
            best_method = MemoryAccessMethod::Abstract;
        }
    }

    // 2. AbstAuto (if hardware reported capability)
    if hart.has_abstauto {
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

    // 3. Sysbus (if hardware reported capability)
    if hart.has_sysbus {
        let res = bench_method(
            &mut hart.dmi,
            &scratch,
            |d, a, s| RiscvMem::sysbus_write(d, a, s).is_ok(),
            |d, a, dst| RiscvMem::sysbus_read(d, a, dst).is_ok(),
        );
        if res.ok_bulk && res.tx_bulk < best_tx {
            best_tx = res.tx_bulk;
            best_method = MemoryAccessMethod::Sysbus;
        }
    }

    let _ = best_tx;
    hart.active_method = best_method;
    let name = match hart.active_method {
        MemoryAccessMethod::Sysbus => "Sysbus",
        MemoryAccessMethod::AbstAuto => "AbstAuto",
        MemoryAccessMethod::Abstract => "Abstract",
        MemoryAccessMethod::Progbuf => "Progbuf",
    };
    crate::glue::gdb_out_rs("Selected memory access method: ");
    crate::glue::gdb_out_rs(name);
    crate::glue::gdb_out_rs("\n");
}

/// Run benchmark against the generic active memory dispatch path.
#[inline(never)]
pub fn run_benchmark2(hart: &mut RiscvHart) -> bool {
    let scratch = init_scratch();

    let active_name = match hart.active_method {
        MemoryAccessMethod::Sysbus => "Sysbus",
        MemoryAccessMethod::AbstAuto => "AbstAuto",
        MemoryAccessMethod::Abstract => "Abstract",
        MemoryAccessMethod::Progbuf => "Progbuf",
    };

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
