/**
 * @file hosted_common.cpp
 * @brief Helpers shared by every hosted executable (any probe back end).
 */

#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "gdb_decoder.h"
#include "hosted_common.h"
#include "hosted_probe.h"

// Keyboard hand-off with the CLI 'q' handler (see ask_probe_choice() in
// bmp_qtserial.cpp). It lives here rather than in the back end because the
// shared main.cpp references it whichever back end is linked in.
static std::atomic<bool> s_stdin_prompt_active{false};
static std::atomic<int> s_stdin_prompt_char{-1};

/** True while a foreground prompt owns the keyboard. */
extern "C" bool hosted_stdin_prompt_is_active()
{
    return s_stdin_prompt_active.load();
}

/** Hand a character read by the caller's key handler to the active prompt. */
extern "C" void hosted_stdin_prompt_push(int ch)
{
    s_stdin_prompt_char.store(ch);
}

/** Take the character handed over by the key handler, -1 when none. */
int hosted_stdin_prompt_take()
{
    return s_stdin_prompt_char.exchange(-1);
}

/** Start / end the keyboard hand-off. */
void hosted_stdin_prompt_begin()
{
    s_stdin_prompt_char.store(-1);
    s_stdin_prompt_active.store(true);
}

void hosted_stdin_prompt_end()
{
    s_stdin_prompt_active.store(false);
    s_stdin_prompt_char.store(-1);
}

// Scratch buffer the probe-side RPC code borrows for one reply
// (`get_temp_buffer_c()` is called from the Rust RPC implementation).
static constexpr size_t TEMP_BUFFER_WORDS = 1024;
static uint32_t s_temp_buffer[TEMP_BUFFER_WORDS];

extern "C" uint8_t *get_temp_buffer_c(uint32_t size)
{
    if (size >= sizeof(s_temp_buffer))
    {
        fprintf(stderr, "Buffer request size %u exceeds %zu\n", size, sizeof(s_temp_buffer));
        exit(1);
    }
    return reinterpret_cast<uint8_t *>(s_temp_buffer);
}

// The GDB replies (`rngdb_send_data_c`) and the runner itself live in Rust now:
// swindle/rs/rs_swindle/src/hosted/qt_runner.rs owns the session.
