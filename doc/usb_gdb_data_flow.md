# USB CDC → GDB Stub: Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        HOST (GDB client)                                │
│  e.g. `gdb` or `OpenOCD` sending "$g" packet over USB CDC/ACM          │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ USB frame
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  RP2040/RP2350 USB hardware (Pico SDK tinyusb stack)                    │
│  - Receives USB frame from host                                         │
│  - Routes to CDC/ACM interface #0 (GDB)                                 │
│  - Fires interrupt → TinyUSB callback                                   │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  lnUsbStack / lnUsbCDC  (C++, esprit USB stack wrapper)                 │
│  - Wraps TinyUSB CDC-ACM in a C++ class                                 │
│  - On data received: calls CdcEventHandler::handle(DataAvailable)       │
│  - This is the `GdbCdcHandler` (Rust struct in usb_gdb.rs)              │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ CdcEvent::DataAvailable
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  GdbCdcHandler::handle()  (Rust, in usb_gdb.rs)                         │
│  - Sets GDB_CDC_DATA_AVAILABLE bit in EventGroup                        │
│  - This wakes up the gdb_task loop                                      │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ EventGroup::set_events()
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  gdb_task()  (C++, in bmp_usb.cpp)                                      │
│  - rngdb_cdc_wait_events() returns with GDB_CDC_DATA_AVAILABLE set      │
│  - Calls rngdb_cdc_process_pending_data()                               │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ FFI call (extern "C")
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  rngdb_cdc_process_pending_data()  (Rust, in usb_gdb.rs)                │
│  → GdbCdc::process_pending_data()                                       │
│  - Borrows the static 1024-byte buffer from GdbCdc struct               │
│  - Loops: calls GdbCdc::read(buffer) until no more data                 │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ GdbCdc::read(&mut buffer)
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  GdbCdc::read()  (Rust, in usb_gdb.rs)                                  │
│  → cdc.read(buffer)  (CdcAcm::read from rust_esprit)                    │
│  - Calls into TinyUSB via rust_esprit CDC abstraction                   │
│  - Returns number of bytes read (or 0 if empty)                         │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ raw bytes in buffer
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  rngdbstub_run(len, ptr)  (Rust, in lib.rs)                             │
│  - Converts raw pointer + length to &[u8] slice                         │
│  - If target is running:                                                │
│      - byte 0x03 → target_halt() (Ctrl-C)                               │
│      - other → log warning                                              │
│  - If target is halted:                                                 │
│      - Calls gdb_main::gdb_process_packet(data)                         │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ &[u8] GDB packet
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  gdb_process_packet()  (C, in blackmagic/src/gdb_main.c)                │
│  - Parses the GDB packet (e.g. "$g" = read registers)                   │
│  - Dispatches to the appropriate handler:                               │
│      'g' → target_regs_read(cur_target, ...)                            │
│      'm' → target_mem32_read(cur_target, ...)                           │
│      'c' → target_halt_resume(cur_target, ...)                          │
│      ...                                                                │
│  - Calls gdb_put_packet_hex/ok/err to send response                     │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ response via gdb_put_packet_*()
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  gdb_put_packet_*()  (C, in blackmagic/src/gdb_packet.c)                │
│  - Encodes response into GDB packet framing ($.../#XX)                  │
│  - Calls rngdb_send_data_c() to send back to host                       │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ FFI call (extern "C")
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  rngdb_send_data_c()  (Rust, in usb_gdb.rs)                             │
│  → GdbCdc::write(data)                                                  │
│  - Loops: calls cdc.write() until all data sent                         │
│  - TinyUSB sends USB frame back to host                                 │
└──────────────────────────┬──────────────────────────────────────────────┘
                           │ USB frame
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                        HOST (GDB client)                                │
│  Receives response packet (e.g. hex register dump)                      │
└──────────────────────────────────────────────────────────────────────────┘
```

## Layer Summary

| Layer | File | Language | Role |
|-------|------|----------|------|
| USB HW | Pico SDK | C | TinyUSB stack, interrupt handling |
| USB wrapper | `lnUsbCDC` / `lnUsbStack` | C++ | esprit abstraction over TinyUSB |
| Event handler | `GdbCdcHandler` in `usb_gdb.rs` | Rust | Converts USB events → EventGroup bits |
| Task loop | `gdb_task()` in `bmp_usb.cpp` | C++ | Waits for events, dispatches to Rust |
| CDC read | `GdbCdc::read()` in `usb_gdb.rs` | Rust | Reads raw bytes from CDC |
| Data processing | `GdbCdc::process_pending_data()` in `usb_gdb.rs` | Rust | Read loop → feeds GDB stub |
| GDB stub entry | `rngdbstub_run()` in `lib.rs` | Rust | Routes to parser or halt handler |
| GDB parser | `gdb_process_packet()` in `gdb_main.c` | C | Parses command, dispatches to target ops |
| Target ops | `target_*.c` in `blackmagic/src/target/` | C | SWD protocol, register/memory access |
| Response | `gdb_put_packet_*()` → `rngdb_send_data_c()` | C → Rust | Encodes response, sends via CDC |

## Key FFI Boundaries

```
C++ gdb_task ──extern "C"──▶ Rust rngdb_cdc_process_pending_data()
                                    │
                                    ├──▶ Rust GdbCdc::read() ──▶ C++ lnUsbCDC
                                    │
                                    └──▶ Rust rngdbstub_run() ──▶ C gdb_process_packet()
                                                                        │
                                                                        └──▶ C rngdb_send_data_c()
                                                                                │
                                                                          extern "C" (Rust)
                                                                                │
                                                                                ▼
                                                                        Rust GdbCdc::write()
                                                                              │
                                                                              ▼
                                                                        C++ lnUsbCDC::write()
```
