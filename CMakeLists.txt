
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
SET(CMAKE_EXPORT_COMPILE_COMMANDS ON)

cmake_minimum_required(VERSION 3.13)
MESSAGE(STATUS "======================")
MESSAGE(STATUS "Starting lnBMP ")
MESSAGE(STATUS "======================")

OPTION(USE_STLINK_PINOUT "Use SWD pins to control target" OFF)
OPTION(USE_GD32F3        "Target GD32F303 chip with 256 flash" OFF)
OPTION(USE_CH32V3x       "Target USE_CH32V3x chip with 256 flash" OFF)


IF(USE_NO_DEFAULT)

ELSE(USE_NO_DEFAULT)
  #SET(USE_CH32V3x True)
    SET(USE_CLANG True)

    SET(USE_CH32V3x        True CACHE INTERNAL "")
    #SET(USE_GD32F3      TRUE CACHE INTERNAL "")
    #SET(USE_CLANG       TRUE CACHE INTERNAL "")
    #SET(LN_LTO "-flto"  CACHE INTERNAL "")
ENDIF(USE_NO_DEFAULT)
IF(USE_CH32V3x)
    SET(USE_CH32v3x_USB_OTG True CACHE INTERNAL "")
ENDIF(USE_CH32V3x)

SET(LN_ENABLE_USBD  True CACHE INTERNAL "")
SET(LN_ENABLE_I2C   FALSE CACHE INTERNAL "")
SET(LN_ENABLE_SPI   FALSE CACHE INTERNAL "")
SET(AF_FOLDER  ${CMAKE_SOURCE_DIR}/lnArduino/)
include(./mcuSelect.cmake)
# Update paths
SET(CMAKE_TOOLCHAIN_FILE ${AF_FOLDER}/lnArduino.cmake)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${AF_FOLDER}/cmake CACHE INTERNAL "")

PROJECT(lnBMP NONE) # the NONE is very important !

enable_language(C CXX ASM) # this is important too!
#
include(./enabled_board.cmake)
#
include(./patch_blackmagic.cmake)

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
    MESSAGE(STATUS "Using lnBMP pinout (PB8/PB9)")
    SET(TGT_PINOUT "default_pinout")
ENDIF()

IF(USE_CLANG)
    MESSAGE(STATUS "Using Clang/llvm toolchain")
    SET(LN_COMPILER "_CLANG" CACHE STRING "")
ELSE()
    MESSAGE(STATUS "Using Gcc toolchain")
    target_link_libraries(lnBMP "-Wl,--allow-multiple-definition") # we have symbols from gcc and symbols from llvm
    SET(LN_COMPILER "_GCC" CACHE STRING "")
ENDIF()    

set_target_properties(lnBMP PROPERTIES OUTPUT_NAME lnBMP_${TGT_PINOUT}_${LN_MCU}_${MCU_SPEED_M}M${LN_COMPILER} )
#
#   Checksum the file...
#
#
IF(USE_CH32V3x)
    add_custom_command(TARGET lnBMP
    POST_BUILD
    COMMAND python3 ${CMAKE_SOURCE_DIR}/scripts/lnCH32Checksum.py  $<TARGET_FILE:lnBMP>.bin $<TARGET_FILE:lnBMP>.ck_bin
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
    COMMENT "Generating checksumed file"
    )
ELSE()
    add_custom_command(TARGET lnBMP
    POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -Obinary $<TARGET_FILE:lnBMP> $<TARGET_FILE:lnBMP>.tmp
    COMMAND python3 ${CMAKE_SOURCE_DIR}/scripts/lnARMChecksum.py  $<TARGET_FILE:lnBMP>.tmp $<TARGET_FILE:lnBMP>.ck_bin
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
    COMMENT "Generating checksumed bin file"
    )

ENDIF(USE_CH32V3x) # Create checksum

#---
