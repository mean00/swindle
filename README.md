
SWINDLE - ARM/Riscv Debug Module
=====

**TLDR : You have a RP2040 board laying around ? you have a debug probe.**

Swindle is a debug probe for embedded device. It uses SWD for Arm chips and RVSWD for CH32v2xx/CH32V3xx chips.

Internally, it uses the incredible [Black Magic Probe](https://black-magic.org/index.html) engine with a custom wrapper.

Swindle runs on RP2040, RP2350, GD32F303CCT6 , CH32V303RCT6. You need 256 kB of flash and >= 48kb of RAM.


![screenshot](assets/web/swindle_demo2.png?raw=true "front")

**A tiny rp2040-zero debugging a full sized RP2040**

(yes i like hot glue)

Summary
=====
- Using BlackMagic engine
- Very portable
- Control part written in Rust
- RP2040 zero boards are dirt cheap :)
- Partial Support for FreeRTOS and RTT out of the box , no configuration
- Infinite breakpoints in RAM
- Only SWD/RVSWD, no Jtag
- Dedicated USB RTT channel on RP2xxx
- (Optional) Custom PCB with voltage translators



Documentation
==============

Look at the [wiki](https://github.com/mean00/swindle/wiki)
   
   
