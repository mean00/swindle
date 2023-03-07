
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
SET(CMAKE_EXPORT_COMPILE_COMMANDS ON)

cmake_minimum_required(VERSION 3.13)
MESSAGE(STATUS "======================")
MESSAGE(STATUS "Starting lnBMP ")
MESSAGE(STATUS "======================")

OPTION(USE_STLINK_PINOUT "Use SWD pins to control target" OFF)

SET(USE_GD32F3 TRUE CACHE INTERNAL "")
SET(USE_CLANG TRUE CACHE INTERNAL "")
SET(LN_LTO "-flto" CACHE INTERNAL "")

SET(LN_ENABLE_USBD True CACHE INTERNAL "")
SET(LN_ENABLE_I2C  FALSE CACHE INTERNAL "")
SET(LN_ENABLE_SPI  FALSE CACHE INTERNAL "")
SET(AF_FOLDER  ${CMAKE_SOURCE_DIR}/lnArduino/)
include(./mcuSelect.cmake)
# Update paths
SET(CMAKE_TOOLCHAIN_FILE ${AF_FOLDER}/lnArduino.cmake)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${AF_FOLDER}/cmake CACHE INTERNAL "")

PROJECT(lnBMP NONE) # the NONE is very important !

enable_language(C CXX ASM) # this is important too!
#

include(applyPatch)
APPLY_PATCH_IF_NEEDED(patched6 blackmagic_small_exception.patch             blackmagic         "use smaller setjmp/long jmp on smaller arm")
APPLY_PATCH_IF_NEEDED(patched8 blackmagic_no_debug_bmp.patch                blackmagic         "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED(patched7 blackmagic_use_custom_target_mem_map.patch   blackmagic      "use heap based mem map")
APPLY_PATCH_IF_NEEDED(patched9 blackmagic_no_libopencm3.patch               blackmagic      "use heap based mem map")
APPLY_PATCH_IF_NEEDED(patched10 blackmagic_dont_use_strtok.patch            blackmagic      "use heap based mem map")
#APPLY_PATCH_IF_NEEDED(patched9 blackmagic_intercept_commands.patch        blackmagic      "intercept gdb commands ")

include_directories(${CMAKE_SOURCE_DIR})
include_directories(${CMAKE_SOURCE_DIR}/include)

SET(LN_ENABLE_RUST ON CACHED "")
IF(USE_STLINK_PINOUT)
    SET(LN_DEBUG_UART 1 CACHE INTERNAL "")
ENDIF()

add_subdirectory(${AF_FOLDER})

# tiny USB configuration
SET(LN_USB_NB_CDC 2) # 2 CDC interfaces
SET(LN_USB_NB_HID 0) # No HID
SET(LN_USB_DFU_RUNTIME 1 CACHE INTERNAL "")
include(${AF_FOLDER}/setup.cmake)
USE_LIBRARY(tinyUsb)

#SET(LN_CUSTOM_LD_SCRIPT ${CMAKE_SOURCE_DIR}/custom_ldscript.ld)

include(${AF_FOLDER}/libraries/tinyUsb/tiny.cmake)

SET(BMP ${CMAKE_SOURCE_DIR}/blackmagic)

GENERATE_GD32_FIRMWARE(lnBMP src/lnBMP.cpp)


ADD_SUBDIRECTORY(lnBlackmagic)
ADD_SUBDIRECTORY(rs)
target_link_libraries(lnBMP lnarduino_c_bindings)
target_link_libraries(lnBMP lnBlackmagic )
target_link_libraries(lnBMP gd32_usb_usbd )
target_link_libraries(lnBMP lnBMP_interface )
target_link_libraries(lnBMP rsbmp)

MESSAGE(STATUS "Summary")
MESSAGE(STATUS "=======")
IF(USE_STLINK_PINOUT)
    MESSAGE(STATUS "STLink style pinout (PA13/PA14)")
    set_target_properties(lnBMP PROPERTIES OUTPUT_NAME lnBMP_stlink_pinout )
ELSE(USE_STLINK_PINOUT)
    MESSAGE(STATUS "lnBMP pinout (PB8/PB9)")
    set_target_properties(lnBMP PROPERTIES OUTPUT_NAME lnBMP_default_pinout)
ENDIF()


