# Instructions for Updating `doc/arch.mmd`

This document defines the architectural guidelines, conventions, and procedures for maintaining and updating [`doc/arch.mmd`](arch.mmd), the canonical architecture diagram of the Swindle project.

---

## 1. Architectural Model Overview

`doc/arch.mmd` models the complete Swindle system, emphasizing the **tripartite codebase division**:

```
+---------------------------------------------------------------------------------+
|                                 GDB / LLDB Client                               |
+---------------------------------------------------------------------------------+
                                         |
                                         v
+---------------------------------------------------------------------------------+
|                       1. Swindle Rust Core (`rs_swindle`)                       |
|   - GDB packet engine, stream automaton, command dispatch (vCont, m, X, g, z)  |
|   - Rust Target Subsystem (`rs_swindle::target` - replaces target.c)            |
|   - Rust Flash Subsystem (`rs_swindle::flash` - replaces target_flash.c)        |
|   - Zero-allocation XML streaming (replaces gdb_main.c)                         |
|   - FreeRTOS introspection & SEGGER RTT polling engine                          |
|   - C-ABI Export Shims (`target::c_adapter.rs`, `flash::c_adapter.rs`)          |
+---------------------------------------------------------------------------------+
                         |                             |
                         | (C-ABI Shims)               | (FFI Bridge)
                         v                             v
+---------------------------------------+   +-------------------------------------+
| 2. Swindle C/C++ Infrastructure Layer |   | 3. Retained Blackmagic C Drivers    |
|    (`swindle/src/`, `swindleio_impl`) |   |    (`blackmagic/src/target/`)       |
| - bmp_helpers.cpp (replaces           |   | - Silicon Probes & Flash Algorithms |
|   exception.c, hex_utils.c,           |   |   (stm32*.c, rp2040.c, rp2350.c,    |
|   maths_utils.c, gdb_reg.c,           |   |   ch32*.c, nrf*.c, lpc*.c)          |
|   stm32_common.c)                     |   | - CPU Architecture Debug            |
| - Diagnostic logger (tc_printf)       |   |   (cortexm.c, riscv32.c,            |
| - Target breakpoint query helpers     |   |   riscv_debug.c, riscv_adi_dtm.c)   |
| - Platform TAP Bit-Bangers            |   | - CoreSight AP/DP & Transports      |
|   (bmp_swdTap_ln.cpp,                 |   |   (adiv5.c, adiv6.c, adiv5_swd.c,   |
|   bmp_rvTap_ln.cpp, bmp_sdiTap_ln.cpp)|   |   spi.c, sfdp.c, jtag_scan.c)       |
| - Hardware reset & pin controls       |   | - Hosted Remoting (protocol_v*.c)   |
|   (bmp_reset_pin.cpp,                 |   | * Pure In-Band Fault Propagation    |
|   bmp_reset_inv.cpp)                  |   |   (setjmp/longjmp completely        |
|                                       |   |    removed)                         |
+---------------------------------------+   +-------------------------------------+
                         \                             /
                          \                           /
                           v                         v
+---------------------------------------------------------------------------------+
|                       Physical Hardware, GPIO & Target MCU                      |
+---------------------------------------------------------------------------------+
```

---

## 2. Layer Structure & Mapping

The diagram is organized into 7 distinct top-down layers:

| Layer | Subgraph Name | Language / Component | Purpose |
|---|---|---|---|
| **Layer 1** | `1. Client Layer` | Client Applications | GDB, LLDB, RTT/Serial terminal consoles. |
| **Layer 2** | `2. Transport Layer` | Transport Adapters | USB CDC (TinyUSB) and TCP/IP socket servers (Hosted). |
| **Layer 3** | `3. Swindle Protocol & Command Engine (Rust)` | Rust (`rs_swindle`) | GDB protocol decoding, command execution, Rust Target/Flash engines, zero-alloc XML streaming, and prefetch read cache. |
| **Layer 4** | `4. Introspection & Services (Rust)` | Rust (`rs_swindle`) | FreeRTOS TCB crawler/stack unwinder, RTT buffer discovery, and RPC subsystem. |
| **Layer 5** | `5. Swindle C/C++ Infrastructure & C-ABI Seam` | Rust Shims + Swindle C/C++ | Rust C-ABI export layer (`c_adapter.rs`), Swindle C/C++ helpers (`bmp_helpers.cpp`), and high-speed platform bit-bang TAP drivers (`swindleio_impl`). |
| **Layer 6** | `6. Blackmagic Upstream Drivers (Retained C, Patched)` | C (`blackmagic/src/target/`) | Retained silicon flash drivers, Cortex-M/RISC-V architecture debug, and CoreSight ADI/SWD/JTAG/SPI transports. All patched for **in-band fault propagation (no setjmp/longjmp)**. |
| **Layer 7** | `7. Physical Target & Hardware Pins` | Hardware & PHY | Target MCU, debug pins (SWCLK, SWDIO, NRST), PIO, and fast GPIO. |

---

## 3. Style & Color Conventions

All nodes in `doc/arch.mmd` must use one of the standard color classes:

```mermaid
classDef rustLang fill:#ffe0b2,stroke:#e65100,stroke-width:2px;        %% Warm Orange: Rust code
classDef swindleCLang fill:#e1f5fe,stroke:#0288d1,stroke-width:2px;    %% Light Blue: Swindle C/C++
classDef blackmagicCLang fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px; %% Soft Green: Patched Blackmagic C
classDef transportLang fill:#f3e5f5,stroke:#7b1fa2,stroke-width:1.5px; %% Lavender: External / Transport
classDef hwLang fill:#eceff1,stroke:#455a64,stroke-width:1.5px;        %% Slate Grey: Physical HW / Pins
```

### Class Assignment Rules
- **Rust code** (`rs_swindle`, `target_xml.rs`, `c_adapter.rs`, etc.): assign `rustLang`.
- **Swindle C/C++** (`bmp_helpers.cpp`, `bmp_interface_c.cpp`, `bmp_swdTap_ln.cpp`, etc.): assign `swindleCLang`.
- **Blackmagic C** (`stm32*.c`, `cortexm.c`, `riscv32.c`, `adiv5.c`, `adiv5_swd.c`, etc.): assign `blackmagicCLang`.
- **Transports & Clients** (GDB, USB CDC, TCP socket): assign `transportLang`.
- **Physical Pins & Targets** (GPIO, PIO, Target MCU): assign `hwLang`.

---

## 4. Mermaid Syntax Guidelines

To ensure `doc/arch.mmd` renders cleanly across GitHub, IDE extensions, and Mermaid CLI (`mmdc`):

1. **Always quote node labels with special characters**:
   - Correct: `CABITarget["target::c_adapter.rs<br/>(Exports target_new, target_mem_read/write)"]`
   - Incorrect: `CABITarget[target::c_adapter.rs (Exports target_new)]`
2. **Use `<br/>` for multiline text inside labels**:
   - Avoid HTML markup other than `<br/>`.
3. **Use simple alphanumeric Node IDs**:
   - Use CamelCase (e.g., `FlashSubsystem`, `SWD_Tap`, `CortexM`).
4. **Group related components into subgraphs**:
   - Always quote the subgraph title: `subgraph CmdModules ["Command Handlers (Rust)"]`.
5. **Keep edge directions consistent**:
   - Ingress and control dispatch flow top-to-bottom (`-->`).
   - Undirected lines (`---`) may be used for bidirectional client/transport streams.

---

## 5. Procedure for Updating the Architecture Diagram

Follow these steps whenever code is migrated, refactored, or added:

### Step 1: Identify What Changed
- **New Rust module replacing C code**: Note the deleted/commented C files in `swindle/swindle_common.cmake` and the new Rust module in `swindle/rs/rs_swindle/`.
- **Patchset cleanup**: When patches in `patches_1.10/` or `cmake/patch_blackmagic.cmake` drop obsolete C file hunks, verify whether those files should be removed from Layer 6.
- **New hardware TAP or board driver**: Check whether it is implemented in `swindle/src/platform/` (Swindle C/C++) or `blackmagic/src/target/` (Blackmagic C).

### Step 2: Update Layer 3 / 4 (Rust Subsystems)
- If new protocol commands, XML formats, or introspection capabilities are added, add nodes to `Layer3` or `Layer4`.
- Explicitly note in the label what C module was replaced, e.g.:
  ```mermaid
  TargetXML["Register XML Generator (target_xml.rs)<br/>(Replaces gdb_main.c XML)"]
  ```

### Step 3: Update Layer 5 (C-ABI Shims & Swindle C/C++)
- If new C symbols are exported from Rust to C drivers, update `RustCABIShims`.
- If new helpers or stubs are added to `bmp_helpers.cpp` or TAP drivers, update `SwindleCHelpers` or `SwindleTAP`.

### Step 4: Update Layer 6 (Blackmagic Upstream C)
- If a Blackmagic C module is no longer compiled (e.g., `target.c`, `target_flash.c`, `command.c`, `gdb_main.c`, `remote.c`):
  - **Remove it** from Layer 6.
  - Ensure its replacement is marked in Layer 3 or Layer 5.
- If architectural behavior in Blackmagic changes (such as the removal of `setjmp`/`longjmp`), update the driver labels to reflect the new calling convention.

### Step 5: Update Node Connections & Styling
- Add or update arrows (`-->`) connecting caller and callee layers.
- Update the `class ...` assignment lists at the bottom of each layer section to maintain color coding.

### Step 6: Verify and Render Test
1. **Syntax Check**: Preview the diagram using an IDE Mermaid previewer or run:
   ```bash
   # If mermaid-cli is available:
   mmdc -i doc/arch.mmd -o /tmp/arch.png
   ```
2. **Build Consistency**: Ensure the diagram matches current build files:
   - Check `BM_TARGET` and `BM_SRC` in `swindle/swindle_common.cmake`.
   - Check active patches in `cmake/patch_blackmagic.cmake`.
   - Check active Rust exports in `swindle/rs/rs_swindle/src/target/c_adapter.rs` and `swindle/rs/rs_swindle/src/flash/c_adapter.rs`.
