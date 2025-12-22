
SWINDLE - ARM/Riscv Debug Module
=====

**TLDR : You have a RP2040 board laying around ? you have a debug probe.**

Swindle is a debug probe for embedded device. It uses SWD for Arm chips and RVSWD for CH32v2xx/CH32V3xx chips.

Internally, it uses the incredible [Black Magic Probe](https://black-magic.org/index.html) engine with a custom wrapper.

Swindle runs on RP2040, RP2350 (experimental), GD32F303CCT6 , CH32V30xxx. You need 256 kB of flash and >= 48kb of RAM.


![screenshot](assets/web/swindle_demo2.png?raw=true "front")

**A tiny rp2040-zero debugging a full sized RP2040**

(yes i like hot glue)

Summary
=====
- Using BlackMagic engine
- Very portable
- Gdb interface written in Rust
- Update over DFU
- RP2040 zero boards are dirt cheap :)
- Partial Support for FreeRTOS and RTT out of the box , no configuration
- Infinite breakpoints in RAM (ARM/RV32)
- Only SWD/RVSWD, no Jtag
- Dedicated USB RTT channel (RP2xxx and CH32V3xx)
- (Optional) Custom PCB with voltage translators
- Breakpoint in flash for target not having HW bkp (some WCH chips)
- Experimental Networked version based on CH32V307 (DHCP + port 2000/20001)



Documentation
==============

Look at the [wiki](https://github.com/mean00/swindle/wiki)
   
   
