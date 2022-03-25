
lnBMP
=====

This is a homegrown port of the Black Magic Probe on top of lnArduino.  

The main goal is that , since it is inside a higher level framework, it is  easier to modify and to add small features.  

The Blackmagic original code is pulled "as-is" through a git submodule.  

Quick FAQ
---------

* Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
* Can i run it on xyz board ? : No, only STM32F103, GD32F103 and GD32F303 are supported (GD32VF103 lacks a usb driver atm)
* Can i use a DFU update on a regular BMP software ? : No you have to flash it through SWD
* Is Jtag supported ? : No, only SWD
* What's the point then ? : It has basic freeRTOS thread support through the normal info thread, thread, etc.. gdb commands

How to build
------------

* Edit platformConfig.cmake to point to your toolchain
* Edit mcuSelect.cmake to pick your MCU (default it is GD32F303)

Then, the usual cmake thing
> mkdir build
> cd build
> cmake .. && make

and then flash build/lnBMP.elf
