
SWINDLE 
=====

A derivative of the black magic probe with rust in it

Works on GD32F3x, CH32v2x and RP2040.
Able to debug ARM through SWD, and CH32V3xx chips (RISCV) through RVSWD

Overview
=====

swindle is a custom port of the amazing Black Magic Probe.

https://github.com/blackmagic-debug/blackmagic


Demo : A tiny rp2040-zero debugging a big-issh CH32V307 eval board :

![screenshot](assets/web/rp2040_ch32.png?raw=true "front")



Why ?
=====
Because it is not tied to Arm (same code fine for CH32V3x/RISCV)
Because it's fun to play with rust.
Because using the RP2040 zero makes a very small & cheap debugger

Additionnaly, the structure of the gdb rust parser is simpler than the one
from blackmagic and easier to modify to add support for things.

Since under the hood FreeRTOS is used, swindle is easy to extend by creating separate standalone threads


Quick FAQ
==================

* Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
* Can i run it on xyz board ? : No, only RP2040, STM32F103, GD32F103, GD32F303 and CH32V3x (maybe CH32V2X)
* Can i debug all the boards : No, only some are enabled (ARM SWD + CH32V RVSWD), but you can easily change that.
* How do i update it ? : Through DFU for all boards except the RP2040
* Is Jtag supported ? : No, only SWD/RVSWD. You can reactivate it if you really need it.


Documentation
==============

Look at the [wiki](https://github.com/mean00/swindle/wiki)
   
   
