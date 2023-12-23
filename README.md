
lnBMP : black magic probe with rust in it
=====

lnBMP is a custom port of the amazing Black Magic Probe.

lnBmp is made of 3 layers :

* Gdb command parser written in rust.
* Blackmagic probe debugger engine, pretty much untouched.
* Low level stuff using lnArduino.
* Limited FreeRTOS support

![screenshot](assets/web/lnBMP.png?raw=true "front")



Why ?
=====
Because it is not tied to Arm (same code fine for CH32V3x/RISCV)
Because it's fun to play with rust.
Because using the RP2040 zero makes a very small & cheap debugger

Additionnaly, the structure of the gdb rust parser is simpler than the one
from blackmagic and easier to modify to add support for things.

Since under the hood FreeRTOS is used, lnBMP is easy to extend by creating separate standalone threads

Quick FAQ
==================

* Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
* Can i run it on xyz board ? : No, only RP2040, STM32F103, GD32F103, GD32F303 and CH32V3x (maybe CH32V2X)
* Can i debug all the boards : No, only some are enabled, but you can easily change that.
* How do i update it ? : Through DFU for all boards except the RP2040
* Is Jtag supported ? : No, only SWD. You can reactivate it if you really need it.

How to build
------------

* Edit platformConfig.cmake to point to your toolchain
* Edit build_default.cmake to change your MCU (USE_GD32F303, USE_CH32V3X, USE_RP2040)

Then, the usual cmake thing
> mkdir build
> cd build
> cmake .. && make

and then dfu update (the 2 scripts for ST/GD and WCH are provided)


**Make sure you flash lnBMP_bootloader! first**

Pinout
==================
GD32/STM32/WCH32V303:
------------------
- SWDIO : PB8
- SWDCLK: PB9
- RESET : PB6
- Uart : PB10 & PB11
- 
RP2040:
------------------
- SWDIO  : GPIO12
- SWDCLK : GPIO13
- RESET  : GPIO11
- Uart   : GPIO4 (Tx), GPIO5 (RX)
   
   
