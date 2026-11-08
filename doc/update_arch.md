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
|          1. Swindle Rust Core (`rs_swindle`, `rs_targets`, wrapper)             |
|   - GDB packet engine, stream automaton, command dispatch (vCont, m, X, g, z)  |
|   - Rust Target Subsystem (`rs_swindle::target` - replaces target.c)            |
|   - Rust Flash Subsystem (`rs_swindle::flash` - replaces target_flash.c)        |
|   - Rust ARM Subsystem (`rs_swindle::arm` - replaces cortexm.c, cortex.c,      |
|     adiv5.c, adiv6.c, adiv5_swd.c)                                              |
|   - Rust RISC-V Subsystem (`rs_swindle::riscv` - replaces riscv32.c,            |
|     riscv_debug.c, riscv_adi_dtm.c)                                             |
|   - Target Driver Crates (`rs_targets`: STM32, RP, NRF, CH32V)                  |
|   - Probe Chains (`rs_swindle_wrapper`: arm_probe_chain.rs, riscv_shim.rs)      |
|   - Zero-allocation XML streaming (replaces gdb_main.c)                         |
|   - FreeRTOS introspection & SEGGER RTT polling engine                          |
|   - Minimal C-ABI Stubs (`target::c_adapter.rs` only; flash & riscv eliminated) |
+---------------------------------------------------------------------------------+
                         |                             |
                         | (Wire & TAP I/O)            | (Hosted Remoting RPC)
                         v                             v
+---------------------------------------+   +-------------------------------------+
| 2. Swindle C/C++ Infrastructure Layer |   | 3. Retained Blackmagic C Code       |
|    (`swindle/src/`, `swindleio_impl`, |   |    (`blackmagic/src/platforms/      |
|     `c_interface`)                    |   |     hosted/`)                       |
| - C interface bridge                  |   | - Hosted Remoting                   |
|   (bmp_interface_c.cpp, bmp_rpc.cpp,  |   |   (bmp_remote.c, protocol_v*.c,    |
|   bmp_stubs_c.cpp)                    |   |    platform.c, cli.c, spi.c)        |
| - Wire transaction counters & memlog  |   | * Zero Blackmagic target or wire    |
|   (ln_swd_tx_count, ln_rv_tx_count)   |   |   drivers in embedded firmware:     |
| - Platform TAP Bit-Bangers            |   |   all silicon probes, flash, CPU    |
|   (bmp_swdTap_ln.cpp,                 |   |   debug, and CoreSight ADI/SWD      |
|   bmp_rvTap_ln.cpp, bmp_sdiTap_ln.cpp)|   |   transports are 100% Rust!         |
| - Hardware reset & pin controls       |   |                                     |
|   (bmp_reset_pin.cpp,                 |   |                                     |
|   bmp_reset_inv.cpp)                  |   |                                     |
| - Board stubs (bmp_disabledBoard.cpp) |   |                                     |
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
| **Layer 2** | `2. Transport Layer` | Transport Adapters | USB CDC (TinyUSB) and TCP/IP socket servers (Hosted, RP2040 W5500). |
| **Layer 3** | `3. Swindle Protocol & Command Engine (Rust)` | Rust (`rs_swindle`, `rs_targets`, `rs_swindle_wrapper`) | GDB protocol decoding, command execution, Rust Target/Flash engines, ARM Cortex-M & CoreSight ADIv5/v6/SWD, RISC-V Hart/Mem/DMI, target driver crates (`rs_targets`), probe chains (`rs_swindle_wrapper`), zero-alloc XML streaming, and prefetch read cache. |
| **Layer 4** | `4. Introspection & Services (Rust)` | Rust (`rs_swindle`) | FreeRTOS TCB crawler/stack unwinder, RTT buffer discovery, and RPC subsystem. |
| **Layer 5** | `5. Swindle C/C++ Infrastructure & C Bridge` | Rust Shims + Swindle C/C++ | Rust C-ABI export stubs (`target::c_adapter.rs`), Swindle C/C++ bridge (`bmp_interface_c.cpp`, `bmp_rpc.cpp`), and high-speed platform bit-bang TAP drivers (`swindleio_impl`). |
| **Layer 6** | `6. Blackmagic Hosted Remoting (Retained C, Hosted Only)` | C (`blackmagic/src/platforms/hosted/`) | Retained hosted remoting client and platform utilities. (Embedded firmware compiles zero Blackmagic C drivers). |
| **Layer 7** | `7. Physical Target & Hardware Pins` | Hardware & PHY | Target MCU, debug pins (SWCLK, SWDIO, NRST), PIO, and fast GPIO. |

---

## 3. Style & Color Conventions

All nodes in `doc/arch.mmd` must use one of the standard color classes. The palette encodes
**code ownership** first: all Swindle-owned code uses the blue family (deep blue = Rust,
pale blue = Swindle C/C++), patched upstream Blackmagic C uses amber.

```mermaid
classDef swindleRust fill:#bbdefb,stroke:#0d47a1,stroke-width:2px;        %% Deep Blue: Swindle Rust
classDef swindleC fill:#e1f5fe,stroke:#0288d1,stroke-width:2px;           %% Pale Blue: Swindle C/C++
classDef blackmagicC fill:#ffe3c2,stroke:#bf360c,stroke-width:2px;        %% Amber: Patched Blackmagic C
classDef external fill:#f3e5f5,stroke:#7b1fa2,stroke-width:1.5px;         %% Lavender: External / Transport
classDef hw fill:#eceff1,stroke:#455a64,stroke-width:1.5px;               %% Slate Grey: Physical HW / Pins
%% Layer (subgraph) tints, same ownership hues
classDef swindleGroup fill:#f4faff,stroke:#5b9bd5,stroke-width:1.5px;     %% Swindle layers (3, 4, 5)
classDef blackmagicGroup fill:#fff7ef,stroke:#bf360c,stroke-width:1.5px;  %% Blackmagic layer (6)
classDef neutralGroup fill:#fafafa,stroke:#90a4ae,stroke-width:1px;       %% Client/Transport/HW layers (1, 2, 7)
```

### Class Assignment Rules
- **Swindle Rust** (`rs_swindle`, `rs_targets`, `rs_swindle_wrapper`, `target_xml.rs`, etc.): assign `swindleRust`.
- **Swindle C/C++** (`bmp_interface_c.cpp`, `bmp_rpc.cpp`, `bmp_swdTap_ln.cpp`, etc.): assign `swindleC`.
- **Blackmagic C** (`bmp_remote.c`, `protocol_v*.c`, `platform.c`, `spi.c`): assign `blackmagicC`.
- **Transports & Clients** (GDB, USB CDC, TCP socket): assign `external`.
- **Physical Pins & Targets** (GPIO, PIO, Target MCU): assign `hw`.
- **Subgraphs** get the matching group tint: `class Layer3 swindleGroup;` (also `Layer4`, `Layer5`),
  `class Layer6 blackmagicGroup;`, `class Layer1/Layer2/Layer7 neutralGroup;`.
- **Legend**: the `Legend` subgraph at the top of `doc/arch.mmd` lists one sample node per class and
  must stay in sync with the `classDef` block above.

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
- **New Rust module replacing C code**: Note the deleted/commented C files in `swindle/swindle_common.cmake` and the new Rust module in `swindle/rs/rs_swindle/` or `swindle/rs/rs_targets/`.
- **Patchset cleanup**: When patches in `patches_1.10/` or `cmake/patch_blackmagic.cmake` drop obsolete C file hunks, verify whether those files should be removed from Layer 6.
- **New hardware TAP or board driver**: Check whether it is implemented in `swindle/src/platform/` (Swindle C/C++) or `swindle/rs/rs_targets/` (Swindle Rust).

### Step 2: Update Layer 3 / 4 (Rust Subsystems)
- If new protocol commands, XML formats, or introspection capabilities are added, add nodes to `Layer3` or `Layer4`.
- Explicitly note in the label what C module was replaced, e.g.:
  ```mermaid
  TargetXML["Register XML Generator (target_xml.rs)<br/>(Replaces gdb_main.c XML)"]
  ```

### Step 3: Update Layer 5 (C Bridge & Swindle C/C++)
- If new C symbols or bridges are exported, update `RustCABIShims` or `SwindleCBridge`.
- If TAP drivers or reset pin controls change, update `SwindleTAP` or `ResetPin`.

### Step 4: Update Layer 6 (Blackmagic Upstream C)
- If a Blackmagic C module is no longer compiled (e.g., all target probes, flash drivers, Cortex-M, and ADIv5 are now in Rust):
  - **Remove it** from Layer 6.
  - Ensure its replacement is marked in Layer 3 or Layer 5.
- Retain only genuinely compiled C modules (e.g. hosted remoting in `blackmagic/src/platforms/hosted/`).

### Step 5: Update Node Connections & Styling
- Add or update arrows (`-->`) connecting caller and callee layers.
- Update the `class ...` assignment lists at the bottom of each layer section to maintain color coding
  (node class + the layer's own `swindleGroup` / `blackmagicGroup` / `neutralGroup` tint).

### Step 6: Verify and Render Test
1. **Syntax Check**: Preview the diagram using an IDE Mermaid previewer or run:
   ```bash
   # If mermaid-cli is available:
   mmdc -i doc/arch.mmd -o /tmp/arch.png
   ```
2. **Build Consistency**: Ensure the diagram matches current build files:
   - Check `BM_TARGET` and `BM_SRC` in `swindle/swindle_common.cmake`.
   - Check active patches in `cmake/patch_blackmagic.cmake`.
   - Check active Rust exports in `swindle/rs/rs_swindle/src/target/c_adapter.rs` (note that `flash::c_adapter.rs` and `riscv::c_adapter.rs` were eliminated).


### Step 7: now data oriented

   The previous steps where about source code dependencies.
   Do the same job resulting in arch_data.mmd (next to arch.mmd).
   The goal of this diagram is to represent the *data* relationship 
   between different structs.
   Also add a small explanation next to the struct to explain what it is
   , what it represents and what it used for. Be brief.

