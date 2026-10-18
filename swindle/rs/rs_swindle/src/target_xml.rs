//! Target XML register description generator
//!
//! Replaces Blackmagic's C-side `target_regs_description()` which used heap
//! `malloc()` and dynamic `snprintf()` calls.
//!
//! Generates XML directly in Rust using composable micro-templates.
//! Eliminates all duplicated register tag definitions and boilerplates.
//!
//! XML document structure produced:
//! ```xml
//! <?xml version="1.0"?>
//! <!DOCTYPE target SYSTEM "gdb-target.dtd">
//! <target>
//!   <architecture>{arch}</architecture>
//!   <feature name="org.gnu.gdb.{feature1}">
//!     <reg name="{name}" bitsize="32".../>
//!   </feature>
//! </target>
//! ```

use crate::bmp::{self, bmp_arch};
use crate::encoder::encoder;

// CORTEXM target option flags (from cortex_internal.h)
pub const CORTEXM_TOPT_FLAVOUR_V6M: u32 = 1 << 1;
pub const CORTEXM_TOPT_FLAVOUR_FLOAT: u32 = 1 << 2;
pub const CORTEXM_TOPT_FLAVOUR_V8M: u32 = 1 << 3;
pub const CORTEXM_TOPT_TRUSTZONE: u32 = 1 << 4;

// RISC-V ISA extensions (from riscv_debug.h)
pub const RV_ISA_EXT_ATOMIC: u32 = 1 << 0;
pub const RV_ISA_EXT_BIT_MANIP: u32 = 1 << 1;
pub const RV_ISA_EXT_COMPRESSED: u32 = 1 << 2;
pub const RV_ISA_EXT_DOUBLE_FLOAT: u32 = 1 << 3;
pub const RV_ISA_EXT_EMBEDDED: u32 = 1 << 4;
pub const RV_ISA_EXT_SINGLE_FLOAT: u32 = 1 << 5;
pub const RV_ISA_EXT_INTEGER: u32 = 1 << 8;
pub const RV_ISA_EXT_DYNAMIC_LANG: u32 = 1 << 9;
pub const RV_ISA_EXT_DECIMAL_FLOAT: u32 = 1 << 11;
pub const RV_ISA_EXT_MUL_DIV_INT: u32 = 1 << 12;
pub const RV_ISA_EXT_PACKED_SIMD: u32 = 1 << 15;
pub const RV_ISA_EXT_QUAD_FLOAT: u32 = 1 << 16;
pub const RV_ISA_EXT_TRANSACT_MEM: u32 = 1 << 19;
pub const RV_ISA_EXT_VECTOR: u32 = 1 << 21;

pub const RV_ISA_EXT_GENERAL_PURPOSE: u32 =
    RV_ISA_EXT_INTEGER | RV_ISA_EXT_MUL_DIV_INT | RV_ISA_EXT_ATOMIC | RV_ISA_EXT_SINGLE_FLOAT | RV_ISA_EXT_DOUBLE_FLOAT;

// Shared XML micro-templates
// Header and outer envelope:
const XML_HEADER: &str = "<?xml version=\"1.0\"?><!DOCTYPE target SYSTEM \"gdb-target.dtd\"><target>  <architecture>";
const ARCH_CLOSE: &str = "</architecture>";
const XML_TARGET_CLOSE: &str = "</feature></target>";

// Feature opening tag prefix: emits `</feature><feature name="org.gnu.gdb.{name}">`
// (The initial feature in the target omits the leading `</feature>`, handled by emit_feature_open).
const FEATURE_GDB_PREFIX: &str = "<feature name=\"org.gnu.gdb.";
const FEATURE_CLOSE_OPEN_GDB: &str = "</feature><feature name=\"org.gnu.gdb.";
const TAG_END: &str = "\">";

// Register attributes and suffixes:
const REG_HEAD: &str = "<reg name=\"";
const BIT32_CLOSE: &str = "\" bitsize=\"32\"/>";
const BIT32_DATA_PTR: &str = "\" bitsize=\"32\" type=\"data_ptr\"/>";
const BIT32_CODE_PTR: &str = "\" bitsize=\"32\" type=\"code_ptr\"/>";
const BIT32_SR_NO: &str = "\" bitsize=\"32\" save-restore=\"no\"/>";
const BIT32_SR_NO_DATA_PTR: &str = "\" bitsize=\"32\" save-restore=\"no\" type=\"data_ptr\"/>";

// Digits table for decimal formatting up to 31 without core::fmt
const DEC_STR: [&str; 32] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
    "10", "11", "12", "13", "14", "15", "16", "17", "18", "19",
    "20", "21", "22", "23", "24", "25", "26", "27", "28", "29",
    "30", "31",
];

/// XML emission sink that either counts bytes or streams a window into the encoder.
pub(crate) enum XmlSink<'a> {
    Count(usize),
    Stream {
        start: usize,
        end: usize,
        offset: usize,
        encoder: &'a mut encoder,
    },
}

impl<'a> XmlSink<'a> {
    #[inline(never)]
    pub fn write(&mut self, chunk: &str) {
        match self {
            XmlSink::Count(total) => {
                *total += chunk.len();
            }
            XmlSink::Stream {
                start,
                end,
                offset,
                encoder,
            } => {
                let chunk_len = chunk.len();
                let chunk_start = *offset;
                let chunk_end = chunk_start + chunk_len;
                if chunk_end > *start && chunk_start < *end {
                    let s_start = if *start > chunk_start { *start - chunk_start } else { 0 };
                    let s_end = if *end < chunk_end { *end - chunk_start } else { chunk_len };
                    encoder.add(&chunk[s_start..s_end]);
                }
                *offset = chunk_end;
            }
        }
    }

    #[inline(never)]
    pub fn write_u32(&mut self, mut n: u32) {
        if n == 0 {
            self.write("0");
            return;
        }
        let mut buf = [0u8; 10];
        let mut i = 10;
        while n > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        if let Ok(s) = core::str::from_utf8(&buf[i..]) {
            self.write(s);
        }
    }
}

/// Helper to emit a register tag:
/// Output: `<reg name="{name}{suffix}" {tail}`
#[inline(never)]
fn emit_reg(sink: &mut XmlSink, name: &str, suffix: &str, tail: &str) {
    sink.write(REG_HEAD);
    sink.write(name);
    if !suffix.is_empty() {
        sink.write(suffix);
    }
    sink.write(tail);
}

/// Helper to transition between features or open a new feature block:
/// If `is_first` is false: `</feature><feature name="org.gnu.gdb.{feature_subname}">`
/// If `is_first` is true:  ` <feature name="org.gnu.gdb.{feature_subname}">`
fn emit_feature_open(sink: &mut XmlSink, feature_subname: &str, is_first: bool) {
    if is_first {
        sink.write(" ");
        sink.write(FEATURE_GDB_PREFIX);
    } else {
        sink.write(FEATURE_CLOSE_OPEN_GDB);
    }
    sink.write(feature_subname);
    sink.write(TAG_END);
}

// --- Cortex-M generator ---
// Emits feature blocks for ARM Cortex-M:
//   1. org.gnu.gdb.arm.m-profile : r0-r12, sp, lr, pc, xpsr
//   2. org.gnu.gdb.arm.m-system  : msp, psp, state
//   3. org.gnu.gdb.arm.m-secext  : (optional TrustZone) msp_ns, psp_ns, msplim_ns, psplim_ns, state_ns, ..._s
//   4. org.gnu.gdb.arm.vfp       : (optional FPU) fpscr, d0..d15 (64-bit float registers)
fn for_each_cortexm_features(target_options: u32, sink: &mut XmlSink) {
    // Feature 1: org.gnu.gdb.arm.m-profile
    // Example: <feature name="org.gnu.gdb.arm.m-profile"><reg name="r0" bitsize="32"/>...</feature>
    emit_feature_open(sink, "arm.m-profile", true);

    // r0..r12 -> `<reg name="r{0..12}" bitsize="32"/>`
    for i in 0..=12 {
        emit_reg(sink, "r", DEC_STR[i], BIT32_CLOSE);
    }

    // sp, lr, pc, xpsr:
    // `<reg name="sp" bitsize="32" type="data_ptr"/>`
    // `<reg name="lr" bitsize="32" type="code_ptr"/>`
    // `<reg name="pc" bitsize="32" type="code_ptr"/>`
    // `<reg name="xpsr" bitsize="32" regnum="25"/>`
    emit_reg(sink, "sp", "", BIT32_DATA_PTR);
    emit_reg(sink, "lr", "", BIT32_CODE_PTR);
    emit_reg(sink, "pc", "", BIT32_CODE_PTR);
    emit_reg(sink, "xpsr", "", "\" bitsize=\"32\" regnum=\"25\"/>");

    // Feature 2: org.gnu.gdb.arm.m-system
    // Example: </feature><feature name="org.gnu.gdb.arm.m-system"><reg name="msp" ...
    emit_feature_open(sink, "arm.m-system", false);
    emit_reg(sink, "msp", "", BIT32_SR_NO_DATA_PTR);
    emit_reg(sink, "psp", "", BIT32_SR_NO_DATA_PTR);
    emit_reg(sink, "state", "", BIT32_SR_NO);

    // Feature 3: org.gnu.gdb.arm.m-secext (TrustZone)
    // Non-secure bank (_ns) first, then secure bank (_s)
    // Example: `<reg name="msp_ns" bitsize="32" save-restore="no" type="data_ptr"/>`
    if (target_options & CORTEXM_TOPT_TRUSTZONE) != 0 {
        emit_feature_open(sink, "arm.m-secext", false);
        for mode in &["_ns", "_s"] {
            for reg in &["msp", "psp", "msplim", "psplim"] {
                emit_reg(sink, reg, mode, BIT32_SR_NO_DATA_PTR);
            }
            emit_reg(sink, "state", mode, BIT32_SR_NO);
        }
    }

    // Feature 4: org.gnu.gdb.arm.vfp (FPU)
    // Cortex-M4F/M7 FPUs: fpscr (32-bit) followed by d0..d15 (64-bit float)
    // Example: `<reg name="fpscr" bitsize="32"/><reg name="d0" bitsize="64" type="float"/>`
    if (target_options & CORTEXM_TOPT_FLAVOUR_FLOAT) != 0 {
        emit_feature_open(sink, "arm.vfp", false);
        emit_reg(sink, "fpscr", "", BIT32_CLOSE);
        for i in 0..16 {
            emit_reg(sink, "d", DEC_STR[i], "\" bitsize=\"64\" type=\"float\"/>");
        }
    }
}

// --- RISC-V GPRs table ---
const RISCV_GPR_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2",
    "fp", "s1", "a0", "a1", "a2", "a3", "a4", "a5",
    "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7",
    "s8", "s9", "s10", "s11", "t3", "t4", "t5", "t6",
];

const RISCV_FPU_GROUPS: [(&str, u8, u32); 4] = [
    ("ft", 8, 33),  // ft0..ft7 -> regnum 33..40
    ("fs", 2, 41),  // fs0..fs1 -> regnum 41..42
    ("fa", 8, 43),  // fa0..fa7 -> regnum 43..50
    ("fs", 10, 51), // fs2..fs11 -> regnum 51..60
];

const RISCV_CSRS: [(&str, &str); 9] = [
    ("mstatus", "896"),
    ("misa", "897"),
    ("mie", "900"),
    ("mtvec", "901"),
    ("mscratch", "960"),
    ("mepc", "961"),
    ("mcause", "962"),
    ("mtval", "963"),
    ("mip", "964"),
];

/// Emits the RISC-V ISA architecture string (e.g. "riscv:rv32imac" or "riscv:rv32ec")
fn emit_riscv_arch(extensions: u32, sink: &mut XmlSink) {
    sink.write("riscv:rv32");

    let is_embedded = (extensions & RV_ISA_EXT_EMBEDDED) != 0;
    sink.write(if is_embedded { "e" } else { "i" });

    let is_g = !is_embedded && ((extensions & RV_ISA_EXT_GENERAL_PURPOSE) == RV_ISA_EXT_GENERAL_PURPOSE);
    if is_g {
        sink.write("g");
        if (extensions & RV_ISA_EXT_QUAD_FLOAT) != 0 {
            sink.write("q");
        }
    } else {
        if (extensions & RV_ISA_EXT_MUL_DIV_INT) != 0 { sink.write("m"); }
        if (extensions & RV_ISA_EXT_ATOMIC) != 0 { sink.write("a"); }
        if (extensions & RV_ISA_EXT_QUAD_FLOAT) != 0 {
            sink.write("q");
        } else if (extensions & RV_ISA_EXT_DOUBLE_FLOAT) != 0 {
            sink.write("d");
        } else if (extensions & RV_ISA_EXT_SINGLE_FLOAT) != 0 {
            sink.write("f");
        }
    }

    if (extensions & RV_ISA_EXT_DECIMAL_FLOAT) != 0 { sink.write("l"); }
    if (extensions & RV_ISA_EXT_COMPRESSED) != 0 { sink.write("c"); }
    if (extensions & RV_ISA_EXT_BIT_MANIP) != 0 { sink.write("b"); }
    if (extensions & RV_ISA_EXT_DYNAMIC_LANG) != 0 { sink.write("j"); }
    if (extensions & RV_ISA_EXT_TRANSACT_MEM) != 0 { sink.write("t"); }
    if (extensions & RV_ISA_EXT_PACKED_SIMD) != 0 { sink.write("p"); }
    if (extensions & RV_ISA_EXT_VECTOR) != 0 { sink.write("v"); }
}

// --- RISC-V generator ---
// Emits feature blocks for RISC-V:
//   1. org.gnu.gdb.riscv.cpu : zero (regnum=0), ra (code_ptr), sp/gp/tp/fp (data_ptr), gprs, pc
//   2. org.gnu.gdb.riscv.fpu : (optional) ft0..ft7, fs0..fs1, fa0..fa7, fs2..fs11, ft8..ft11, fflags, frm, fcsr
//   3. org.gnu.gdb.riscv.csr : mstatus (896), misa (897), mie (900), mtvec (901), ...
fn for_each_riscv_features(extensions: u32, sink: &mut XmlSink) {
    // Feature 1: org.gnu.gdb.riscv.cpu
    // Example: <feature name="org.gnu.gdb.riscv.cpu"><reg name="zero" bitsize="32" regnum="0"/>...
    emit_feature_open(sink, "riscv.cpu", true);

    // 16 GPRs (RV32E) or 32 GPRs (RV32I)
    let gprs_count = if (extensions & RV_ISA_EXT_EMBEDDED) != 0 { 16 } else { 32 };
    for i in 0..gprs_count {
        let name = RISCV_GPR_NAMES[i];
        let tail = match i {
            0 => "\" bitsize=\"32\" regnum=\"0\"/>",
            1 => BIT32_CODE_PTR,
            2 | 3 | 4 | 8 => BIT32_DATA_PTR,
            _ => BIT32_CLOSE,
        };
        emit_reg(sink, name, "", tail);
    }

    emit_reg(sink, "pc", "", BIT32_CODE_PTR);

    // Feature 2: org.gnu.gdb.riscv.fpu (Single float extension)
    // Emits registers: ft0..ft7, fs0..fs1, fa0..fa7, fs2..fs11, ft8..ft11 with explicit regnum:
    // Example: `<reg name="ft0" bitsize="32"  regnum="33"/>`
    if (extensions & RV_ISA_EXT_SINGLE_FLOAT) != 0 {
        emit_feature_open(sink, "riscv.fpu", false);
        for &(pfx, count, start_regnum) in &RISCV_FPU_GROUPS {
            let start_idx = if pfx == "fs" && count == 10 { 2 } else { 0 };
            for i in 0..count {
                let reg_num = start_idx + i as u32;
                let regnum = start_regnum + i as u32;
                sink.write(REG_HEAD);
                sink.write(pfx);
                sink.write_u32(reg_num);
                sink.write("\" bitsize=\"32\" type=\"ieee_single\" regnum=\"");
                sink.write_u32(regnum);
                sink.write("\" group=\"float\"/>");
            }
        }
        // ft8..ft11 -> regnum 61..64
        for i in 0..4u32 {
            sink.write(REG_HEAD);
            sink.write("ft");
            sink.write_u32(8 + i);
            sink.write("\" bitsize=\"32\" type=\"ieee_single\" regnum=\"");
            sink.write_u32(61 + i);
            sink.write("\" group=\"float\"/>");
        }
        emit_reg(sink, "fflags", "", "\" bitsize=\"32\" type=\"int\" regnum=\"66\" group=\"float\" save-restore=\"no\"/>");
        emit_reg(sink, "frm", "", "\" bitsize=\"32\" type=\"int\" regnum=\"67\" group=\"float\" save-restore=\"no\"/>");
        emit_reg(sink, "fcsr", "", "\" bitsize=\"32\" type=\"int\" regnum=\"68\" group=\"float\" save-restore=\"no\"/>");
    }

    // Feature 3: org.gnu.gdb.riscv.csr
    // Example: `<reg name="mstatus" bitsize="32" regnum="896"  save-restore="no"/>`
    emit_feature_open(sink, "riscv.csr", false);
    for &(name, num_str) in &RISCV_CSRS {
        sink.write(REG_HEAD);
        sink.write(name);
        sink.write("\" bitsize=\"32\" regnum=\"");
        sink.write(num_str);
        sink.write("\"  save-restore=\"no\"/>");
    }
}

/// Unified document generator:
/// Wraps architecture and feature emitters inside the standard GDB XML target document:
///
/// Output:
///   `<?xml version="1.0"><!DOCTYPE target SYSTEM "gdb-target.dtd"><target>  <architecture>{arch}</architecture>`
///   `{features}`
///   `</feature></target>`
fn for_each_target_xml_chunk(sink: &mut XmlSink) {
    if !bmp::bmp_attached() {
        return;
    }

    match bmp::bmp_get_arch() {
        bmp_arch::BMP_ARCH_ARM => {
            sink.write(XML_HEADER);
            sink.write("arm");
            sink.write(ARCH_CLOSE);
            for_each_cortexm_features(bmp::bmp_get_target_options(), sink);
            sink.write(XML_TARGET_CLOSE);
        }
        bmp_arch::BMP_ARCH_RISCV => {
            sink.write(XML_HEADER);
            let (_, exts) = bmp::bmp_get_riscv_info().unwrap_or((32, RV_ISA_EXT_INTEGER | RV_ISA_EXT_COMPRESSED));
            emit_riscv_arch(exts, sink);
            sink.write(ARCH_CLOSE);
            for_each_riscv_features(exts, sink);
            sink.write(XML_TARGET_CLOSE);
        }
        _ => {}
    }
}

/// Compute total length of the XML description by running the generator in counting mode.
pub fn get_target_xml_len() -> usize {
    let mut sink = XmlSink::Count(0);
    for_each_target_xml_chunk(&mut sink);
    match sink {
        XmlSink::Count(n) => n,
        _ => 0,
    }
}

/// Send the XML range `[start_address..start_address + length]` through the GDB packet encoder.
pub fn send_target_xml_chunk(e: &mut encoder, start_address: usize, length: usize) {
    let mut sink = XmlSink::Stream {
        start: start_address,
        end: start_address + length,
        offset: 0,
        encoder: e,
    };
    for_each_target_xml_chunk(&mut sink);
}
