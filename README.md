
lnBMP
=====

This is a homegrown port of the Black Magic Probe on top of lnArduino.  

The main goal is that , since it is inside a higher level framework, it is  easier to modify and to add small features, in my case basic support for FreeRTOS.

The Blackmagic original code is pulled "as-is" through a git submodule, and slightly patched by the build process.  

Quick FAQ
---------

* Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
* Can i run it on xyz board ? : No, only STM32F103, GD32F103 and GD32F303 are supported (GD32VF103 lacks a usb driver atm)
* Can i debug all the boards : No, only some are enabled, but you can easily change that.
* Can i use a DFU update on a regular BMP software ? : Yes but /!\ be careful of the pinout used /!\
* Is Jtag supported ? : No, only SWD

How to build
------------

* Edit platformConfig.cmake to point to your toolchain
* Edit mcuSelect.cmake to pick your MCU (default it is STM32F103 with 128kB of flash, ~ bluepill)

Then, the usual cmake thing
> mkdir build
> cd build
> cmake .. && make

and then flash build/lnBMP.elf

How to enable FreeRTOS support in your project
-----------------------------------------------
Make sure you have configUSE_TRACE_FACILITY enabled in FreeRTOSconfig.h
Copy  the following files to your project :
  - lnArduino/include/lnFreeRTOSDebug.h
  - lnArduino/src/lnFreeRTOSDebug.cpp 

The latest is actually mostly C code, you can just rename it to C and slightly modify it if need be.

Make absolutely sure the symbol freeRTOSDebug is present in your final executable.
You can do that by doing a dummy call to lnGetFreeRTOSDebug() to pull a reference to that.

Now you have access to "info thread", "thread x" etc...

_Careful:_  it works fairlty well for cortex M0/3/4 without FPU, not so much for Cortex M4 with FPU enabled.

Why do i need the lnFreeRTOSDebug ?
-----------------------------------
Depending on the way freeRTOS is compiled (including FreeRTOSConfig.h options), the structure representing the internals of freeRTOS will change slightly. That file exports the executable configuration to lnBMP so it works without guesswork.


Default SWD Pinout
-------------------
- SWDIO : PB4
- SWDCLK: PB3
- RESET : PB6

Default Uart Pinout : 
-----------------------
- PB10
- PB11
   

