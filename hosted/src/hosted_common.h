/**
 * @file hosted_common.h
 * @brief Helpers shared by every hosted executable (any probe back end).
 */

#pragma once

/**
 * @name Keyboard hand-off with a foreground prompt
 *
 * `main.cpp` reads the terminal to quit on 'q'; while a prompt owns the
 * keyboard (the USB probe-selection prompt) the character is handed over
 * instead of being acted upon.
 */
/**@{*/
/** True while a foreground prompt owns the keyboard. */
extern "C" bool hosted_stdin_prompt_is_active();
/** Hand a character read by the key handler to the active prompt. */
extern "C" void hosted_stdin_prompt_push(int ch);
/** Take the character handed over by the key handler, -1 when none. */
int hosted_stdin_prompt_take();
/** Start / end the keyboard hand-off. */
void hosted_stdin_prompt_begin();
void hosted_stdin_prompt_end();
/**@}*/
