
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
SET(CMAKE_EXPORT_COMPILE_COMMANDS ON)

cmake_minimum_required(VERSION 3.13)
MESSAGE(STATUS "======================")
MESSAGE(STATUS "Starting swindle ")
MESSAGE(STATUS "======================")

OPTION(USE_STLINK_PINOUT "Use SWD pins to control target" OFF)
OPTION(USE_GD32F3        "Target GD32F303 chip with 256 flash" OFF)
OPTION(USE_CH32V3x       "Target USE_CH32V3x chip with 256 flash" OFF)

#
SET(BMP_VERSION_BRANCH 1.10)
SET(BMP_PATCH_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/patches_${BMP_VERSION_BRANCH})
SET(LNBMP_TOP_FOLDER ${CMAKE_SOURCE_DIR}/blackmagic)

#-------------
# Options
#-------------
IF(USE_NO_DEFAULT)

ELSE(USE_NO_DEFAULT)
  include(./build_default.cmake)
ENDIF(USE_NO_DEFAULT)

IF(USE_CH32V3x)
    SET(USE_CH32v3x_USB_OTG True CACHE INTERNAL "")
ENDIF(USE_CH32V3x)

SET(LN_ENABLE_USBD  TRUE  CACHE INTERNAL "")
SET(LN_ENABLE_I2C   FALSE CACHE INTERNAL "")
SET(LN_ENABLE_SPI   FALSE CACHE INTERNAL "")
SET(LN_ENABLE_UART  TRUE  CACHE INTERNAL "")
SET(LN_ENABLE_RUST  TRUE  CACHE INTERNAL "")

SET(LN_USB_BUFFER_SIZE 256 CACHE INTERNAL "") # increase usb cdc buffer size

SET(AF_FOLDER  ${CMAKE_SOURCE_DIR}/lnArduino)
include(./mcuSelect.cmake)

SET(CMAKE_TOOLCHAIN_FILE ${AF_FOLDER}/lnArduino.cmake)
SET(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${CMAKE_CURRENT_SOURCE_DIR}/cmake ${AF_FOLDER}/cmake CACHE INTERNAL "")

PROJECT(swindle NONE) # the NONE is very important !

enable_language(C CXX ASM) # this is important too!
# Generate version
STRING(TIMESTAMP  timestamp "20%y-%m-%d")
include(swindle_version)
GEN_VERSION_FILE()

#
include(all_boards)
#
include(patch_blackmagic)

include_directories(${CMAKE_SOURCE_DIR})
include_directories(${CMAKE_SOURCE_DIR}/include)



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

include(${AF_FOLDER}/libraries/tinyUsb/tiny.cmake)

SET(BMP ${CMAKE_SOURCE_DIR}/blackmagic)

IF (USE_RP2040)
        SET(SOURCES src/swindle_rp2040.cpp)
        USE_LIBRARY(WS2812_rp_single)
ELSE()
        SET(SOURCES src/swindle.cpp)
ENDIF()

GENERATE_GD32_FIRMWARE(swindle ${SOURCES})
HASH_GD32_FIRMWARE(swindle)    


ADD_SUBDIRECTORY(swindle)
target_link_libraries(swindle lnarduino_c_bindings)
target_link_libraries(swindle libswindle )
target_link_libraries(swindle swindle_interface )
target_link_libraries(swindle rsbmp)

#
#
#
IF(USE_CH32V3x)
    SET(TGT_NAME "CH32V3x")
ELSEIF(USE_GD32F3)
    SET(TGT_NAME "GD32F3")
ELSE()
    SET(TGT_NAME "STM32F1")
ENDIF()
#
#
#
MESSAGE(STATUS "=======")
MESSAGE(STATUS "Summary")
MESSAGE(STATUS "=======")


math(EXPR MCU_SPEED_M "${LN_MCU_SPEED}/1000000")

MESSAGE(STATUS "Built for ${LN_ARCH}/${LN_MCU} with ${LN_MCU_FLASH_SIZE} flash / ${LN_MCU_RAM_SIZE} ram")
MESSAGE(STATUS "          MCU running at ${MCU_SPEED_M} Mhz")

IF(USE_STLINK_PINOUT)
    MESSAGE(STATUS "STLink style pinout (PA13/PA14)")
    SET(TGT_PINOUT "stlink_pinout")
ELSE()
    MESSAGE(STATUS "Using swindle pinout (PB8/PB9)")
    SET(TGT_PINOUT "default_pinout")
ENDIF()

IF(USE_CLANG)
    MESSAGE(STATUS "Using Clang/llvm toolchain")
    SET(LN_COMPILER "_CLANG" CACHE STRING "")
ELSE()
    MESSAGE(STATUS "Using Gcc toolchain")
    target_link_libraries(swindle "-Wl,--allow-multiple-definition") # we have symbols from gcc and symbols from llvm
    SET(LN_COMPILER "_GCC" CACHE STRING "")
ENDIF()    

IF(USE_RP2040)
    set_target_properties(swindle PROPERTIES OUTPUT_NAME swindle_RP2040${LN_COMPILER} )
ELSE()
    set_target_properties(swindle PROPERTIES OUTPUT_NAME swindle_${TGT_PINOUT}_${LN_MCU}_${MCU_SPEED_M}M${LN_COMPILER} )
ENDIF()    

#---
