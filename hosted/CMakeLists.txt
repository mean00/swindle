cmake_minimum_required(VERSION 3.00)
PROJECT(lnBMPHosted NONE) # the NONE is very important !

enable_language(C CXX ASM) # this is important too!
SET(LN_BMP_HOSTED True)
SET(BMP ${CMAKE_SOURCE_DIR}/../blackmagic)
SET(ARDUINO_GD32_FREERTOS ${CMAKE_SOURCE_DIR}/../lnArduino)

include( ${ARDUINO_GD32_FREERTOS}/embeddedPrintf.cmake)

include_directories(include)
include_directories(${BMP}/)
include_directories(${BMP}/src/include)
include_directories(${BMP}/src/platforms/pc)
include_directories(${BMP}/src/platforms/hosted)
include_directories(${BMP}/src/target)
include_directories(${BMP}/../lnBlackmagic/include )
SET(H ${CMAKE_CURRENT_SOURCE_DIR}/src)
#
#
ADD_SUBDIRECTORY(../lnBlackmagic lnBMP)
ADD_DEFINITIONS("-DPC_HOSTED=1")
ADD_DEFINITIONS("-DENABLE_DEBUG=1")

SET(SRC src/main.cpp src/lnDebug.cpp src/stubs.cpp ../lnArduino/embedded_printf/printf.c)
ADD_EXECUTABLE(lnBMP_hosted ${SRC})
target_link_libraries(lnBMP_hosted lnBlackmagic embeddedPrintf usb-1.0 ftdi1 )
