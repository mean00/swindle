/**
 * @file esp_socket_rust.h
 * @brief C ABI the hosted Rust SocketRunner uses to drive the Qt socket layer.
 *
 * The runner itself (slot bits, GDB/relay logic) is Rust: see
 * swindle/rs/rs_swindle/src/hosted/qt_runner.rs. Qt provides bytes and events.
 */
#pragma once

#include <stdbool.h>
#include <stdint.h>

extern "C" {

/** Socket event callback, one call per Qt socket event. */
typedef void (*ln_socket_event_cb)(uint32_t event, void *arg);

/* --- the local listening socket (lnSocketQt) ----------------------------- */

/** Create and listen on `port`; NULL when the port cannot be bound. */
void *ln_socket_open(uint16_t port, ln_socket_event_cb cb, void *arg);
/** Read what is available: at most `cap` bytes, the rest stays buffered here. */
int ln_socket_read(void *sock, uint8_t *dst, uint32_t cap, uint32_t *n);
int ln_socket_write(void *sock, uint32_t n, const uint8_t *data, uint32_t *done);
int ln_socket_flush(void *sock);
int ln_socket_write_buffer_available(void *sock, uint32_t *n);
int ln_socket_disconnect_client(void *sock);
void ln_socket_close(void *sock);

/* --- the link to a network probe (swindle_hosted_net relay) --------------- */

/** Connect to the probe; NULL when it cannot be reached. */
void *probe_link_open(const char *host, uint16_t port);
bool probe_link_is_open(void *link);
int probe_link_read(void *link, int32_t wait_ms, uint8_t *dst, int32_t cap);
bool probe_link_write(void *link, const uint8_t *data, int32_t size);
void probe_link_close(void *link);

/* --- runner lifetime, driven by the hosted GDB server thread -------------- */

/** `probe` NULL: the GDB core serves the session; otherwise relay to the probe. */
void *hosted_runner_new(void *event_group, uint16_t port, const char *probe, uint16_t probe_port);
bool hosted_runner_start(void *runner);
void hosted_runner_process(void *runner, uint32_t events);
/** Drain the socket (and pump the probe link, if any). */
void hosted_runner_poll(void *runner);
bool hosted_runner_is_connected(void *runner);
void hosted_runner_disconnect(void *runner);
/** GDB replies: the BMP core emits them through rngdb_send_data_c(). */
void hosted_runner_write(void *runner, const uint8_t *data, uint32_t len);
void hosted_runner_flush(void *runner);
void hosted_runner_free(void *runner);
}
