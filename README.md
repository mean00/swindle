
lnBMP : black magic probe with rust in it
=====

lnBMP is a custom port of the amazin Black Magic Probe.
lnBmp is made of 3 layers :

* a gdb command parser written in rust.
* the blackmagic probe debugger engine.
* low level stuff using lnArduino.

Why ?
Because it's fun to play with rust.
Additionnaly, the structure of the gdb rust parser is simpler than the one
from blackmagic and easier to modify to add support for things.

Quick FAQ
==================

* Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
* Can i run it on xyz board ? : No, only STM32F103, GD32F103, GD32F303 and CH32V3x.
* Can i debug all the boards : No, only some are enabled, but you can easily change that.
* Can i use a DFU update on a regular BMP software ? : Yes but /!\ be careful of the pinout used /!\
* Is Jtag supported ? : No, only SWD. 

How to build
------------

* Edit platformConfig.cmake to point to your toolchain
* Edit mcuSelect.cmake to pick your MCU (default it is GD32F303 with 256kB of flash!)

Then, the usual cmake thing
> mkdir build
> cd build
> cmake .. && make

and then flash build/lnBMP.elf


Make sure you also flash lnBMP_bootloader!

Pinout
==================

Default SWD Pinout
-----------------------
- SWDIO : PB8
- SWDCLK: PB9
- RESET : PB6

Default Uart Pinout : 
-----------------------
- PB10
- PB11
   
You can also use the STLink pintout by calling cmake with -DUSE_STLINK_PINOUT=1 i.e.
> mkdir build
> cd build
> cmake -DUSE_STLINK_PINOUT=1 .. && make

In that case, the pinout is :

STLink SWD Pinout
-------------------
- SWDIO : PA13
- SWDCLK: PA14
- RESET : PB4

STLink Uart Pinout : 
-----------------------
- PA8
- PA9
   