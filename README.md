
This is a homegrown port of the Black Magic Probe on top of lnArduino.

The main goal is that , since it is inside a higher level framework, it is  easier to modify and add small features.

The Blackmagic original code is pulled "as-is" through a git submodule.

= Quick Faq =

Is it better than vanilla Black Magic Probe ? : No. It supports less targets and is less robust.
Can i run it on xyz board ? : No, only STM32F103, GD32F103 and GD32F303 are supported (GD32V103 lacks a usb driver atm)
Can i use a DFU update on a regular BMP software ? : No you have to flash it through SWD
Is Jtag supported ? : No, only SWD

= How to build =
Edit platformConfig.cmake to point to your toolchain
Edit mcuSelect.cmake to pick your MCU (by default it is GD32F303)
mkdir build
cd build
cmake ..

and flash build/lnBMP.elf
