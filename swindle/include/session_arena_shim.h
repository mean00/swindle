/*
 * session_arena_shim.h - route Blackmagic's session-scoped allocations to the
 * static session arena instead of the FreeRTOS heap (see malloc_used.md).
 *
 * Force-included (-include, see swindle/swindle_target.cmake) into every
 * Blackmagic C source of libswindle, i.e. the ADIv5/Cortex/RISC-V target layer
 * that creates the per-session objects (DP, AP, priv, target_s, ram/flash/
 * command descriptors, breakwatch nodes).  This header only renames the three
 * allocator calls; the declarations are in session_arena.h, which is also what
 * the C++ side (swindle/src/bmp_helpers.cpp) includes, because there the macros
 * must not be in force:
 *
 *   * the libc headers that declare malloc/calloc/free have to be seen BEFORE
 *     the macros are defined, otherwise their prototypes would be rewritten
 *     (see the note below); a later #include of them is a no-op;
 *   * the macros must not be active while another header declares malloc /
 *     free: esprit/include/esprit.h declares its own pair with _NOTHROW /
 *     __attribute_malloc__, and esprit/include/lnRingBuffer.h has a member
 *     function called free().  Hence "Blackmagic only, never esprit.h".
 *
 * With SWINDLE_SESSION_ARENA=OFF the same symbols are provided by
 * rs_swindle/src/session_arena_stubs.rs and simply forward to the heap, so
 * nothing here needs to be conditional.
 */
#ifndef SESSION_ARENA_SHIM_H
#define SESSION_ARENA_SHIM_H

/*
 * Every libc header that declares malloc/calloc/free has to be seen BEFORE the
 * macros are defined: the declarations themselves must not be renamed, and once
 * a header is included, its guard makes any later #include a no-op, so nothing
 * re-declares them afterwards.  newlib spreads them over stdlib.h, string.h
 * (`free`, for __malloc_like), malloc.h and wchar.h.
 */
#include <stdlib.h>
#include <string.h>
#include <malloc.h>
#include <wchar.h>

#include "session_arena.h"

#define calloc(n, s) swindle_session_calloc((n), (s))
#define malloc(s) swindle_session_malloc((s))
#define free(p) swindle_session_free((p))

#endif /* SESSION_ARENA_SHIM_H */

