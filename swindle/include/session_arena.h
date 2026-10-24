/*
 * session_arena.h - the C side of the session arena (see malloc_used.md,
 * "Session arena").
 *
 * Declares the entry points implemented by rs_swindle/src/session_arena.rs
 * (or by session_arena_stubs.rs when SWINDLE_SESSION_ARENA=OFF, in which case
 * they forward to the heap).  Two users:
 *
 *   * swindle/src/bmp_helpers.cpp, which names the functions explicitly: it is
 *     C++, so it cannot take the macros of session_arena_shim.h (`new` cannot
 *     be renamed) and it pulls in headers that declare malloc/free themselves
 *     (esprit.h) or have a member function called free() (lnRingBuffer.h);
 *   * session_arena_shim.h, which is force-included into the blackmagic C
 *     sources of libswindle and expands calloc/malloc/free to these functions.
 *
 * Contract, as for the C library: a null return means `malloc(0)`,
 * `calloc(0, ...)` or a `count * size` overflow.  An exhausted arena or heap is
 * fatal (logged, then panic), so the `if (!p)` checks at the call sites only
 * ever fire for the zero cases.
 *
 * Blocks come back `ALIGN`-aligned (8) and zero filled, which is what every
 * blackmagic allocation site wants: it either calls calloc, or memsets/copies
 * over the whole block.  The zero fill holds on the heap fallback too (arena
 * unarmed, SWINDLE_SESSION_ARENA=OFF), so no call site can tell which branch
 * served it; see session_arena.rs, "# Zero fill".
 */
#ifndef SESSION_ARENA_H
#define SESSION_ARENA_H

#include <stddef.h>

#ifdef __cplusplus
extern "C"
{
#endif

    /* calloc(n, size): n * size zeroed bytes from the session pool. */
    void *swindle_session_calloc(size_t n, size_t size);

    /* malloc(size): size zeroed bytes from the session pool.  Zeroing costs
     * nothing here - the region is wiped at the start of the session and a bump
     * block is always fresh. */
    void *swindle_session_malloc(size_t size);

    /* free(p): gives the bytes back when p is the top block of a region (a bump
     * region can only reclaim in LIFO order), ignores any other arena pointer,
     * forwards everything else to the heap. */
    void swindle_session_free(void *p);

    /* Transport region slot, for the only session objects C++ allocates with
     * `new`: the riscv_dmi_s of the RVSWD/SDI templates. */
    void *swindle_session_transport_alloc(size_t size);

    /* Wipe both regions and arm the arena.  Called at the end of
     * target_list_free(), i.e. at the start of every scan. */
    void swindle_session_open(void);

#ifdef __cplusplus
}
#endif

#endif /* SESSION_ARENA_H */
