<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/web/swindle_logo3.png">
  <img alt="Swindle Debug Probe" src="assets/web/swindle_logo3.png" width="100%">
</picture>


<h1 align="center">🔧 SWINDLE — ARM / RISC-V Debug Probe</h1>

<p align="center">
  <b>Turn any cheap dev board into a full-featured debug probe.</b>
</p>

<blockquote>
  <strong>TL;DR</strong> — Got an RP2040 board lying around? You've got a debug probe.<br>
  Swindle is a <strong>multi-platform debug probe</strong> for embedded devices, supporting <strong>SWD</strong> (ARM Cortex-M), <strong>RVSWD</strong> (RISC-V CH32V2xx/CH32V3xx), and <strong>SDI</strong> (WCH CH32V00x). Originally based on <a href="https://black-magic.org/index.html">Black Magic Probe</a>, the firmware has been converted to <strong>Rust</strong> with a decoupled, multi-transport architecture.
</blockquote>

---

## ✨ Features

| Category | Details |
|----------|---------|
| **🦀 Rust Core** | Target drivers, stub runners, GDB/RTT protocol, and transport layers rewritten in **Rust** (`no_std`) |
| **🔌 Protocols** | SWD (ARM) + RVSWD (RISC-V) + SDI (WCH CH32V00x) — no JTAG |
| **🎯 Breakpoints** | Infinite breakpoints in RAM (ARM/RV32) + soft flash breakpoints for targets without HW breakpoints (some WCH chips) |
| **📡 RTOS Support** | **FreeRTOS** awareness and **SEGGER RTT** out of the box — no configuration needed |
| **💾 DFU Update** | Firmware updates over USB DFU — no programmer required |
| **📺 RTT Channel** | Dedicated USB Tx only RTT channel (RP2xxx & CH32V3xx) |
| **🌐 Networking** | Experimental networked version based on **CH32V307** or RP2040+W5500 (DHCP + port 2000/20001) |
| **🔋 Low Cost** | RP2040-Zero boards are dirt cheap! |
| **🔧 Voltage Translation** | Optional custom PCB with voltage level translators |

---

## 🧩 Supported Hardware

### Probe Hosts (runs Swindle)

You need **≥256 KB flash** and **≥48 KB RAM**.

| Platform | Status | Notes |
|----------|--------|-------|
| **RP2040** (e.g. RP2040-Zero) | ✅ Stable | The most affordable option |
| **RP2040 + W5500** | 🧪 Experimental | Ethernet |
| **RP2350** (e.g. RP2350-Zero) | 🧪 Experimental | |
| **GD32F303CCT6** (48/64-pin) | ✅ Stable | |
| **CH32V30xxx** | ✅ Stable | RISC-V based host |
| **Custom PCB** | 🧩 Optional | With voltage translators, mosfet on reset, ws2812 |

### Debug Targets

| Architecture | Protocol | Device Families |
|-------------|----------|-----------------|
| **ARM Cortex-M** | SWD | RP2040, RP2350, STM32, GD32, and many ARM Cortex-M with SWD |
| **RISC-V** | RVSWD / SDI | WCH CH32V2xx, CH32V3xx (RVSWD), CH32V00x (SDI) |

---

## 🚀 Getting Started

### 1. Flash the firmware

Pick the pre-built firmware for your board or [build from source](#-building).

> **Flashing via DFU (RP2040):** Hold `BOOTSEL`, plug in USB, then copy the `.uf2` file.

### 2. Connect to your target

```
Probe SWDIO  →  Target SWDIO
Probe SWCLK  →  Target SWCLK
Probe GND    →  Target GND
Probe NRST   →  Target NRST   (optional, recommended)
Probe TX     →  Target Uart TX    (optional)
```

'see wiki for exact pinout'
### 3. Start debugging

```bash
# Connect with GDB
gdb-multiarch -ex "target extended-remote /dev/ttyACM0" -ex "mon swdp_scan" -ex "attach 1" your_firmware.elf
```

Or use any GDB frontend (VS Code, CLion, etc.) — just point it to the serial port.

---

## 🔨 Building

This project uses **CMake** with presets for each target platform.

There is a devpod/docker build environment included.


### Prerequisites

- CMake ≥ 3.21
- ARM GCC toolchain (`arm-none-eabi-gcc`) or LLVM/Clang
- RISC-V toolchain (for CH32V targets)
- Rust toolchain (for the GDB interface)

### Configure & Build

```bash
# Configure for your target
cmake --preset rp2040          # RP2040-Zero
cmake --preset rp2350          # RP2350-Zero
cmake --preset gd32f303_48     # GD32F303 48-pin
cmake --preset gd32f303_64     # GD32F303 64-pin
cmake --preset ch32v307        # CH32V307 (networked)

# Build
cmake --build --preset rp2040
```

> **Note:** Some presets have variants with inverted NRST (`*_inv`), with WS2812 RGB led.
> See `CMakePresets.json` for the full list.

| Preset | Description |
|--------|-------------|
| `rp2040` | RP2040-Zero (USB) |
| `rp2040_inv` | RP2040-Zero with inverted NRST |
| `rp2350` | RP2350-Zero (USB, experimental) |
| `gd32f303_48` | GD32F303 48-pin (USB) |
| `gd32f303_64` | GD32F303 64-pin (USB) |
| `gd32f303_48_inv_ws2812` | GD32F303 48-pin (USB) +WS2812 +inv RST |
| `gd32f303_64_inv_ws2812` | GD32F303 64-pin (USB) +WS2812 +inv RST |
| `ch32v307` | CH32V30x (USB) |
| `ch32v307_inv_ws2812` | CH32V30x (USB) +ws2812 + inv RST|
| `ch32v307_net` | CH32V307 (Ethernet, experimental) |
| `rp2040_zero_w5500` | rp2040+w5500 (Ethernet, experimental) |

---

## 📘 Documentation

Full documentation is available on the **[Swindle Wiki](https://github.com/mean00/swindle/wiki)** — covering:

- 🔧 Hardware setup and wiring guides
- 🖥️ GDB integration and troubleshooting
- ⚙️ Advanced configuration
- 🧪 RTOS-aware debugging
- 🌐 Networked probe setup

---

## 🏗️ Project Architecture

Unlike the original Black Magic Probe (monolithic C firmware with single-threaded polling), Swindle's probe engine has been rewritten in **Rust** (`no_std`) with a layered, multi-transport architecture. Upstream `blackmagic/` C sources are no longer compiled.

### Key Architectural Changes

- **Higher-level transport abstraction, not bit-level**: Abstracts at the word/transaction level (SWD DP/AP, RVSWD, WCH SDI) rather than generic bit-level JTAG/SWD, enabling faster transfers and platform-specific acceleration (RP2040/RP2350 PIO state machines, fast GPIO).
- **Stricter, smaller codebase (32-bit only)**: Dropping JTAG, 64-bit RISC-V, and legacy targets allows tighter, strongly-typed `no_std` Rust code with less boilerplate and fewer runtime branches.
- **Two dedicated driver lines (ARM & RISC-V) with ad-hoc APIs**: Separate `arm` and `riscv` subsystems, probe chains, and target traits tailored to each architecture instead of forcing both through a single generic C `target` struct.
- **CRC32 & target-side flash stubs**: Runs flash programming and CRC32 verification on the target MCU to speed up `gdb compare-sections` and flashing.
- **Static-first memory & session arena**: Replaces scattered `malloc`/`free` calls with static `.bss` allocations, zero-alloc XML/flash buffers, and a bump-allocated session arena that resets cleanly on every scan to avoid heap fragmentation.
- **Software breakpoints in flash or RAM**: Supports software breakpoints in both RAM (unlimited breakpoints) and flash (for targets without hardware breakpoint units, such as some WCH chips), alongside a 16-byte prefetch line cache to reduce wire round-trips.
- **Schema-generated RPC**: A single TOML schema (`rpc_protocol.toml`) generates both probe and host RPC endpoints using compact CBOR (no hex expansion), allowing the same Rust target drivers to run on-probe or on a PC (`swindle_hosted`) over USB/network.
- **RTOS-backed, multi-transport I/O**: Runs on FreeRTOS via `esprit` instead of a single-threaded polling loop, enabling concurrent USB CDC ports (GDB, UART bridge, logger) as well as TCP/IP networking (CH32V307, RP2040 + W5500).
- **Explicit error propagation (no `setjmp`/`longjmp`)**: Replaces Black Magic Probe's `setjmp`/`longjmp` exception mechanism with structured `Result`-based error propagation across wire, target, and flash layers.
- **Native RTOS & RTT introspection**: FreeRTOS TCB crawling/stack unwinding (ARM Cortex-M & RV32/CH32) and SEGGER RTT polling are built directly into the Rust GDB engine.

---

## 🖼️ Gallery

### RP2040 + CH32V

<p align="center">
  <img src="assets/web/rp2040_ch32.png" alt="RP2040 debugging CH32V" width="400">
</p>

### System Block View

```mermaid
flowchart TD
    classDef client fill:#f3e5f5,stroke:#7b1fa2,stroke-width:1.5px,color:#212121
    classDef rust fill:#bbdefb,stroke:#0d47a1,stroke-width:2px,color:#0d1b2a
    classDef cpp fill:#e1f5fe,stroke:#0288d1,stroke-width:1.5px,color:#0d1b2a
    classDef hw fill:#eceff1,stroke:#455a64,stroke-width:1.5px,color:#212121
    classDef grp fill:#f8fbff,stroke:#90caf9,stroke-width:1px

    subgraph Clients ["Host & Transports"]
        GDB["GDB / LLDB & RTT Terminal"]
        Transports["USB CDC · TCP/IP (W5500 / CH32V307) · Hosted CBOR RPC"]
    end

    subgraph RustCore ["Swindle Rust Core (no_std · swindle/rs)"]
        Engine["GDB / RTT Engine<br/>Packet I/O · FreeRTOS · SW Breakpoints · Read Cache · Session Arena"]
        ArmLine["ARM Line<br/>SWD · ADIv5/6 · Cortex-M"]
        RiscvLine["RISC-V 32 Line<br/>RVSWD · SDI · DM/DMI"]
        Drivers["Target & Flash Drivers (rs_targets)<br/>STM32 · RP2040/RP2350 · nRF · CH32V · CRC32 Stubs"]
    end

    subgraph Platform ["Platform & TAP Layer (swindle/src + esprit)"]
        HAL["FreeRTOS + HAL (esprit)<br/>USB · Net · Reset · VDD ADC"]
        TAP["High-Speed Word/Transaction TAPs<br/>RP2xx PIO · Fast GPIO"]
    end

    Target["Target MCU<br/>ARM Cortex-M · RISC-V RV32"]

    GDB <--> Transports
    Transports <--> Engine
    Engine --> ArmLine
    Engine --> RiscvLine
    ArmLine --> Drivers
    RiscvLine --> Drivers
    ArmLine --> TAP
    RiscvLine --> TAP
    Transports -.-> HAL
    HAL --- TAP
    TAP <-->|"SWD / RVSWD / SDI"| Target

    class GDB,Transports client
    class Engine,ArmLine,RiscvLine,Drivers rust
    class HAL,TAP cpp
    class Target hw
    class Clients,RustCore,Platform grp
```

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!
Feel free to check the [issues page](https://github.com/mean00/swindle/issues).

---

## 📄 License

This project is a derivative of [Black Magic Probe](https://black-magic.org/index.html), and the corresponding parts are licensed under **GPL-3.0**. The Rust conversion and architectural refactoring were assisted by **Gemini** and **DeepSeek**.
