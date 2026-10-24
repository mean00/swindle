//! Size of the session arena regions (`malloc_used.md`, "Session arena").
//!
//! Kept apart from `session_arena.rs` because the numbers are the seam's shared
//! constant: the implementation sizes its two `.bss` regions with them, the stub
//! half (`session_arena_stubs.rs`) reports them through `mon arena`, and they are
//! what the probe's RAM budget has to allow for (`SWINDLE_SESSION_ARENA_RAM_KIB`
//! in the top-level CMakeLists.txt).  A change here is a change to that budget.

/// Bytes reserved for the per-target session objects: the `target_s`, the
/// RAM/Flash/command descriptors, the per-target privs and the breakwatch nodes.
///
/// Measured worst case on the probes is well under this (472 B for an RVSWD
/// RISC-V session); the rest is headroom, because exhausting the region aborts
/// the probe instead of silently falling back to the heap.
pub const POOL_SIZE: usize = 1728;

/// Bytes reserved for the transport: the probe DP plus one DP+AP pair per
/// discovered device (`adiv5_swd.c` allocates a DP per multidrop instance),
/// i.e. 84 + 6 * (84 + 48) = 876 B, plus slack.
///
/// It is a separate region because it is a singleton: a large target pool must
/// never be able to starve the DP/AP chain, and vice versa.
pub const TRANSPORT_SIZE: usize = 896;

/// Alignment of every arena block and of the regions themselves.
///
/// Blackmagic's session objects hold pointers and `uint64_t` counters, and the
/// Cortex-M4 faults on an unaligned `LDRD`/`STRD`, so 8 is the strictest
/// requirement.  The static assert below keeps the `#[repr(align(8))]` of the
/// regions in `session_arena.rs` in step with it.
pub const ALIGN: usize = 8;

// A `Region` is a byte array with an 8-byte alignment, so the guarantee the
// blocks rely on is exactly this equality.
const _: () = assert!(ALIGN == 8);
