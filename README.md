
SWINDLE - ARM/Riscv Debug Module
=====

Swindle is a hardware debugger connecting over Arm-SWD or WCH RvSWD.
It can debug a lot of ARM chip and the CH32V3xx chips from WCH.

Internally, it is a derivative of the incredible [black magic probe](ttps://black-magic.org/index.html) with some modifications.

Swindle runs on GD32F303CCT6 , CH32V303RCT6 and RP2040 (> 1MB flash)
Other models will work, you must have enough flash & ram (256 kB flash / 48 or 64 kb RAM)

Perfect to have a cheap debugger based on a RP2040 (normal or zero) 

Demo : A tiny rp2040-zero debugging a big-issh CH32V307 eval board :

![screenshot](assets/web/rp2040_ch32.png?raw=true "front")



Why ?
=====
- Because it is not tied to Arm (same code fine for CH32V3x/RISCV).
- Because it's fun to play with rust.
- Because using the RP2040 zero makes a very small & cheap debugger.
- Because i wanted native support for CH32 chips and FreeRTOS.
- Because it is built using CMake (at the time the BMP was using plain Makefiles).



Quick FAQ
==================

* Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
* Can i run it on xyz board ? : No, only RP2040,  GD32F303 and CH32V3x (maybe CH32V2X)
* Can i debug all the boards : No, only some are enabled (ARM SWD + CH32V RVSWD). This is mostly a size issue.
* How do i update it ? : Through DFU for all boards except the RP2040
* Is Jtag supported ? : No, only SWD/RVSWD. You can reactivate it if you really need it.


Documentation
==============

Look at the [wiki](https://github.com/mean00/swindle/wiki)
   
   
