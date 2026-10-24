//! Session-scoped static arena (see `malloc_used.md`, "Session arena").
//!
//! Every object blackmagic allocates while a debug session is live - the ADIv5
//! DP/AP chain, the Cortex-M or RISC-V priv, the `target_s`, the RAM/Flash/
//! command descriptors, the breakwatch nodes - is short lived: it is created
//! during `swd_scan`/`rvswdp_scan`/`sdi_scan` and destroyed by
//! `target_list_free()`.  Handing those objects to `heap_4` fragments the probe
//! heap a little more on every connect/disconnect cycle for no benefit, so they
//! are carved out of two `.bss` regions instead.
//!
//! # Mechanism
//!
//! The C side needs no patch: `include/session_arena_shim.h` `#define`s
//! `calloc`/`malloc`/`free` to the `swindle_session_*` entry points below, and
//! `swindle/swindle_target.cmake` force-includes it (`-include`) into the
//! blackmagic `.c` sources of `libswindle`; `bmp_helpers.cpp`, the other holder
//! of session allocations, includes the declarations (`session_arena.h`) and
//! names the `swindle_session_*` functions explicitly.  The macros are confined
//! to that scope because they must never be live in a TU that sees
//! `esprit/include/esprit.h` (which declares `malloc`/`free` itself) or
//! `lnRingBuffer.h` (which has a member function called `free()`).  The libc
//! headers that declare the three functions are included first by the shim, so
//! no prototype is ever rewritten - only the call sites expand.
//!
//! # Regions
//!
//! * `POOL` - bump allocated, holds the per-target objects.
//! * `TRANSPORT` - the SWD/ADIv/RVSWD transport: the probe DP plus one DP+AP
//!   pair per discovered device.  It is a singleton, so it is a separate
//!   region: a large-ish target pool can never starve the transport, and the
//!   transport can never be starved by target objects.
//!
//! Both sizes live in `session_arena_size.rs`.
//!
//! # Lifetime
//!
//! One lifetime for both regions, driven from Rust: `swindle_session_open()`
//! wipes them and arms the arena.  It is called at the end of
//! `target::c_adapter::target_list_free()`, which every scan calls *before* it
//! creates anything (`adiv5_swd_scan()`, and the RVSWD/SDI templates) - by then
//! `priv_free` has run, so the AP/DP chain of the previous session has been
//! released (`cortex_priv_free` -> `adiv5_ap_unref` -> `adiv5_dp_unref`) and
//! both regions are dead.  There is no `close()`: everything the arena serves
//! dies with the target list, so one reset point is enough, and the probe never
//! has to distinguish "inside a session" from "idle" (`'D'` keeps the list, so
//! `attach 1` after a detach must keep working).
//!
//! Before the first scan the arena is unarmed and the `swindle_session_*`
//! allocators forward to the heap, zeroed there too (see "# Zero fill").
//!
//! # Zero fill
//!
//! Every block these entry points return is zero filled, on both branches.
//! `alloc_from` has no choice: a rewound bump block can still hold a previous
//! session's object, so it zeroes what it hands out.  The heap fallback (the
//! unarmed case, and everything that is not session memory) zeroes as well, so
//! that neither a `calloc` site nor the `riscv_dmi_s` - which used to be `new
//! riscv_dmi_s`, i.e. value initialised - can tell which branch served it.
//! `malloc` returning zeroed memory is stricter than C, at the cost of a
//! `memset` over blocks of a few hundred bytes that the callers usually write
//! immediately anyway.
//!
//! # Reclamation
//!
//! A region is a strict bump allocator: memory comes back only as a whole, when
//! the next scan wipes it.  `swindle_session_free()` on an arena pointer
//! therefore reclaims nothing in general - with one exception, the LIFO case
//! that actually happens: freeing the block at the top of a region rewinds the
//! bump pointer to it.  The transient per-session objects (`breakwatch_set()` /
//! `breakwatch_clear()`, target descriptions) are allocated and released one at
//! a time, so that keeps a long session from filling the region with dead nodes.
//! Any other order, and a double `free()`, only count.
//!
//! The wipe zeroes both regions, so a use-after-detach of an arena object lands
//! on zeros instead of recycled neighbouring objects.
//!
//! # Failure policy
//!
//! An exhausted region, and a heap allocation that comes back null, are both
//! fatal: they are logged (`mon redirect 1` makes it visible) and then panic,
//! which the firmware's panic handler turns into `deadEnd(55)`.  There is
//! deliberately no fallback to the heap - that would silently move the session
//! objects back to the allocator this exists to keep them away from - and the
//! two sizes (`session_arena_size.rs`) are chosen with enough headroom
//! (`mon arena` shows the peaks) that a well-behaved session never gets near the
//! limit.

use core::ffi::c_void;
use core::ptr;

use crate::SyncCell;
use crate::session_arena_size::{ALIGN, POOL_SIZE, TRANSPORT_SIZE};

/// Whether this build serves session allocations from the arena.
///
/// `mon arena` prints the statistics only when this is true; the stub half of
/// the seam answers `false`.
pub const BUILT_IN: bool = true;

/// Backing storage for one region, with the alignment the blocks need.
///
/// A plain `static [u8; N]` would be align-1: `adiv5_dp_s` starts with a
/// pointer and `target_s`/the privs contain `uint64_t` fields, and the
/// Cortex-M4 faults on an unaligned `LDRD`/`STRD`, so the region itself has to
/// be at least `ALIGN`-aligned for every block inside it to be.
#[repr(align(8))]
struct Region<const N: usize>(#[allow(dead_code)] [u8; N]);

static mut POOL: Region<POOL_SIZE> = Region([0; POOL_SIZE]);
static mut TRANSPORT: Region<TRANSPORT_SIZE> = Region([0; TRANSPORT_SIZE]);

/// Bump state of one region.
///
/// One `SyncCell` per field rather than one `SyncCell<Bump>`: the allocator is
/// single threaded (the GDB task owns the target list) but the statics have to
/// be `Sync`.
struct Bump {
    /// Bytes handed out so far: the offset the next block starts at, once
    /// aligned up.
    used: SyncCell<usize>,
    /// Highest `used` since the last `open()`; reported by `mon arena`.
    peak: SyncCell<usize>,
    /// Offset of the block handed out last, or `NO_BLOCK`.  A `free()` of
    /// exactly that block rewinds `used` to it (see `free_from`).
    last: SyncCell<usize>,
}

/// No block handed out since the last rewind, so no `free()` can rewind.
const NO_BLOCK: usize = usize::MAX;

impl Bump {
    const fn new() -> Self {
        Self {
            used: SyncCell::new(0),
            peak: SyncCell::new(0),
            last: SyncCell::new(NO_BLOCK),
        }
    }

    /// Back to offset 0, forget the top block.
    fn reset(&self) {
        self.used.set(0);
        self.peak.set(0);
        self.last.set(NO_BLOCK);
    }
}

static POOL_BUMP: Bump = Bump::new();
static TRANSPORT_BUMP: Bump = Bump::new();

/// Number of `free()` calls that landed on arena memory.
static FREES: SyncCell<u32> = SyncCell::new(0);

/// Subset of `FREES` that also gave the memory back (it was the top block).
static REWINDS: SyncCell<u32> = SyncCell::new(0);

/// Whether the arena currently serves allocations.
static ARMED: SyncCell<bool> = SyncCell::new(false);

// The heap.  On the firmware these are esprit's `sysAlloc`/`sysFree` wrappers
// (`esprit/src/systemAlloc.cpp`), on the hosted build the host heap, so the
// unarmed path behaves exactly like the pre-arena code in both cases.
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// Snapshot of the arena state, for `mon arena`.
#[derive(Clone, Copy)]
pub struct ArenaStats {
    pub pool_used: usize,
    pub pool_size: usize,
    pub pool_peak: usize,
    pub transport_used: usize,
    pub transport_size: usize,
    pub transport_peak: usize,
    /// `free()` calls that hit arena memory.
    pub frees: u32,
    /// How many of those rewound their region.
    pub rewinds: u32,
    pub armed: bool,
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Log an exhausted region and abort the probe.
///
/// Nothing can be reported after the panic (the handler is `deadEnd(55)`), so
/// the numbers go to the logger first: `mon redirect 1` before the session makes
/// them visible.
fn out_of_arena(region: &str, capacity: usize, used: usize, size: usize) -> ! {
    crate::bmpwarning!(
        "arena: {} region full: {} + {} > {} B\n",
        region,
        used,
        size,
        capacity
    );
    panic!("session arena region full");
}

/// Log a failed heap allocation and abort the probe.
fn out_of_heap(size: usize) -> ! {
    crate::bmpwarning!("arena: heap allocation of {} B failed\n", size);
    panic!("heap exhausted");
}

/// `malloc()` on the heap, for the allocations the arena does not serve.
///
/// Zero filled, like everything else these entry points return: the arena has
/// no choice (a rewound or reused block can still hold a previous session's
/// object, and `calloc` sites must not see it), so the heap fallback zeroes too
/// rather than letting the `calloc` contract depend on which branch was taken.
/// Unarmed only ever happens at boot, before the first scan arms the arena.
unsafe fn heap_alloc(size: usize) -> *mut c_void {
    let block = unsafe { malloc(size) };
    if block.is_null() {
        out_of_heap(size);
    }
    unsafe { ptr::write_bytes(block.cast::<u8>(), 0, size) };
    block
}

/// `free()` on the heap.
unsafe fn heap_free(p: *mut c_void) {
    unsafe { free(p) };
}

/// Bump `size` zeroed bytes out of the region at `base`, which is `capacity`
/// bytes long and whose bump state is `bump`.
///
/// The block is `ALIGN`-aligned and zero filled - what `calloc` needs, and what
/// the blackmagic `malloc` sites do by hand right after.  A request that does
/// not fit is fatal, see `out_of_arena`.
unsafe fn alloc_from(
    region: &str,
    base: *mut u8,
    capacity: usize,
    bump: &Bump,
    size: usize,
) -> *mut c_void {
    let start = align_up(bump.used.get(), ALIGN);
    let end = start + size;
    if end > capacity {
        out_of_arena(region, capacity, start, size);
    }
    let block = unsafe { base.add(start) };
    unsafe { ptr::write_bytes(block, 0, size) };
    bump.used.set(end);
    bump.last.set(start);
    if end > bump.peak.get() {
        bump.peak.set(end);
    }
    block.cast::<c_void>()
}

/// True when `p` points into the region at `base`.
fn contains(base: *mut u8, capacity: usize, p: *mut c_void) -> bool {
    let addr = p as usize;
    let start = base as usize;
    addr >= start && addr < start + capacity
}

/// `free()` of an arena pointer: reclaim it when it is the top block of the
/// region, otherwise only count it.
///
/// Returns false when `p` is not in this region at all, so the caller can try
/// the other one and finally the heap.
unsafe fn free_from(base: *mut u8, capacity: usize, bump: &Bump, p: *mut c_void) -> bool {
    if !contains(base, capacity, p) {
        return false;
    }
    let offset = p as usize - base as usize;
    FREES.set(FREES.get() + 1);
    if bump.last.get() == offset {
        // The top block: give its bytes back, including the alignment padding
        // that preceded it.
        bump.used.set(offset);
        bump.last.set(NO_BLOCK);
        REWINDS.set(REWINDS.get() + 1);
    }
    true
}

/// Wipe both regions and reset the per-session statistics.
unsafe fn wipe() {
    unsafe {
        ptr::write_bytes(ptr::addr_of_mut!(POOL) as *mut u8, 0, POOL_SIZE);
        ptr::write_bytes(ptr::addr_of_mut!(TRANSPORT) as *mut u8, 0, TRANSPORT_SIZE);
    }
    POOL_BUMP.reset();
    TRANSPORT_BUMP.reset();
    FREES.set(0);
    REWINDS.set(0);
}

/// Zeroed `size` bytes from the target pool, or from the heap while unarmed.
///
/// Shared by `swindle_session_malloc` and `swindle_session_calloc`: a bump block
/// is always fresh and zeroed by `alloc_from`, so `calloc` needs no work of its
/// own beyond the overflow check.
fn pool_alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    if !ARMED.get() {
        return unsafe { heap_alloc(size) };
    }
    unsafe {
        alloc_from(
            "pool",
            ptr::addr_of_mut!(POOL) as *mut u8,
            POOL_SIZE,
            &POOL_BUMP,
            size,
        )
    }
}

/// Zeroed `size` bytes from the transport region, or from the heap while
/// unarmed.
fn transport_alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    if !ARMED.get() {
        return unsafe { heap_alloc(size) };
    }
    unsafe {
        alloc_from(
            "transport",
            ptr::addr_of_mut!(TRANSPORT) as *mut u8,
            TRANSPORT_SIZE,
            &TRANSPORT_BUMP,
            size,
        )
    }
}

/// `malloc()`: `size` zeroed bytes from the target pool.
///
/// This is the C `calloc`/`malloc` entry point: the macros of
/// `swindle/include/session_arena_shim.h` expand the blackmagic call sites to it
/// (force-included into the libswindle `.c` sources), and `bmp_helpers.cpp`
/// calls it by name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_malloc(size: usize) -> *mut c_void {
    pool_alloc(size)
}

/// `calloc()`: `count` zeroed elements of `size` bytes.
///
/// The block is zeroed whichever branch serves it (`alloc_from` zeroes its
/// blocks, `heap_alloc` zeroes the heap), so all this adds is C's `calloc`
/// overflow rule: a `count * size` overflow is a caller bug, not an exhausted
/// arena, and returns null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_calloc(count: usize, size: usize) -> *mut c_void {
    match count.checked_mul(size) {
        Some(total) => pool_alloc(total),
        None => ptr::null_mut(),
    }
}

/// `free()`: rewinds the region when `p` is its top block, ignores any other
/// arena pointer, and forwards everything else to the heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let pool = ptr::addr_of_mut!(POOL) as *mut u8;
    let transport = ptr::addr_of_mut!(TRANSPORT) as *mut u8;
    let reclaimed = unsafe {
        free_from(pool, POOL_SIZE, &POOL_BUMP, p)
            || free_from(transport, TRANSPORT_SIZE, &TRANSPORT_BUMP, p)
    };
    if !reclaimed {
        unsafe { heap_free(p) };
    }
}

/// Zeroed `size` bytes from the transport region, or from the heap while
/// unarmed.
///
/// Used by the RVSWD/SDI templates, which used `new riscv_dmi_s` before: the
/// `riscv_dmi_s` is the only session object allocated from C++ rather than
/// through `calloc`, so it is the only one the header macros cannot see.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_transport_alloc(size: usize) -> *mut c_void {
    transport_alloc(size)
}

/// Wipe both regions and start serving allocations.
///
/// Called at the end of `target_list_free()`, before a scan repopulates the
/// target list: at that point every object of the previous session - targets,
/// privs and the DP/AP chain - has already been released, so both regions can
/// be reused from offset 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_open() {
    unsafe { wipe() };
    ARMED.set(true);
}

/// Current arena state, for `mon arena`.
pub fn stats() -> ArenaStats {
    ArenaStats {
        pool_used: POOL_BUMP.used.get(),
        pool_size: POOL_SIZE,
        pool_peak: POOL_BUMP.peak.get(),
        transport_used: TRANSPORT_BUMP.used.get(),
        transport_size: TRANSPORT_SIZE,
        transport_peak: TRANSPORT_BUMP.peak.get(),
        frees: FREES.get(),
        rewinds: REWINDS.get(),
        armed: ARMED.get(),
    }
}
