
SET(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
SET(CMAKE_EXPORT_COMPILE_COMMANDS ON)

CMAKE_MINIMUM_REQUIRED(VERSION 3.13)
MESSAGE(STATUS "======================")
MESSAGE(STATUS "Starting swindle ")
MESSAGE(STATUS "======================")

OPTION(USE_STLINK_PINOUT "Use SWD pins to control target" OFF)
OPTION(USE_GD32F3        "Target GD32F303 chip with 256 flash" OFF)
OPTION(USE_CH32V3x       "Target USE_CH32V3x chip with 256 flash" OFF)
OPTION(USE_RP_ZERO       "Change RGB LED to map RP2040 board" OFF)
OPTION(USE_RP_CARRIER    "Use RP2040 carrier board" OFF)

#
SET(BMP_VERSION_BRANCH 1.10)
SET(BMP_PATCH_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/patches_${BMP_VERSION_BRANCH})
SET(LNBMP_TOP_FOLDER ${CMAKE_SOURCE_DIR}/blackmagic)

#-------------
# Options
#-------------
IF(NOT USE_NO_DEFAULT)
  INCLUDE(./build_default.cmake)
ENDIF()

IF(USE_CH32V3x)
  SET(USE_CH32v3x_USB_OTG True CACHE INTERNAL "")
ENDIF()
#___________________________________________________
SET(LN_ENABLE_USBD  TRUE  CACHE INTERNAL "")
SET(LN_ENABLE_I2C   FALSE CACHE INTERNAL "")
SET(LN_ENABLE_SPI   FALSE CACHE INTERNAL "")
SET(LN_ENABLE_UART  TRUE  CACHE INTERNAL "")
SET(LN_ENABLE_RUST  TRUE  CACHE INTERNAL "")

# tiny USB configuration
SET(LN_USB_NB_CDC 3) # 2 CDC interfaces
SET(LN_USB_NB_HID 0) # No HID
SET(LN_USB_DFU_RUNTIME 1 CACHE INTERNAL "")

SET(LN_USB_BUFFER_SIZE 256 CACHE INTERNAL "") # increase usb cdc buffer size
#___________________________________________________

#SET(USE_RP2040_PURE_RAM True CACHE INTERNAL "")
#___________________________________________________
SET(AF_FOLDER  ${CMAKE_SOURCE_DIR}/lnArduino)
INCLUDE(./mcuSelect.cmake)

SET(CMAKE_TOOLCHAIN_FILE ${AF_FOLDER}/lnArduino.cmake)
SET(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${CMAKE_CURRENT_SOURCE_DIR}/cmake ${AF_FOLDER}/cmake CACHE INTERNAL "")

PROJECT(swindle NONE) # the NONE is very important !

ENABLE_LANGUAGE(C CXX ASM) # this is important too!
# Generate version
STRING(TIMESTAMP  timestamp "20%y-%m-%d")
INCLUDE(swindle_version)
GEN_VERSION_FILE()

#
INCLUDE(all_boards)
#
INCLUDE(patch_blackmagic)

INCLUDE_DIRECTORIES(${CMAKE_SOURCE_DIR})
INCLUDE_DIRECTORIES(${CMAKE_SOURCE_DIR}/include)

ADD_SUBDIRECTORY(${AF_FOLDER})

INCLUDE(${AF_FOLDER}/setup.cmake)
USE_LIBRARY(tinyUsb)

INCLUDE(${AF_FOLDER}/libraries/tinyUsb/tiny.cmake)
#__________________________________________________________________
#
#
#
MACRO(TUNE_NAME OPTN NAME)
  IF(${OPTN})
    STRING(APPEND TGT_NAME "_${NAME}")
  ENDIF()
ENDMACRO()
#


SET(BMP ${CMAKE_SOURCE_DIR}/blackmagic)

IF(USE_RP2040)
  RP_PIO_GENERATE( ${CMAKE_SOURCE_DIR}/src/ws.pio ${CMAKE_BINARY_DIR}/ws.h)
  IF(USE_RP_ZERO)
    TUNE_NAME(USE_RP_ZERO zero)
    SET(ZERO_FLAGS _zero)
  ENDIF()
  IF(USE_RP_CARRIER)
    TUNE_NAME(USE_RP_CARRIER carrier)
  ENDIF()
  SET(SOURCES src/swindle_rp2040${ZERO_FLAGS}.cpp ${CMAKE_BINARY_DIR}/ws.h)
ELSE()
  SET(SOURCES src/swindle.cpp)
ENDIF()

GENERATE_GD32_FIRMWARE(swindle ${SOURCES})
HASH_GD32_FIRMWARE(swindle)


ADD_SUBDIRECTORY(swindle)
TARGET_LINK_LIBRARIES(swindle lnarduino_c_bindings)
TARGET_LINK_LIBRARIES(swindle libswindle)
TARGET_LINK_LIBRARIES(swindle swindle_interface)
TARGET_LINK_LIBRARIES(swindle rsbmp)

#
TUNE_NAME(USE_CH32V3x CH32V3x)
TUNE_NAME(USE_GD32F3  GD32F3)
TUNE_NAME(USE_RP2040  RP2040)
#
#
#
MESSAGE(STATUS "=======")
MESSAGE(STATUS "Summary")
MESSAGE(STATUS "=======")


MATH(EXPR MCU_SPEED_M "${LN_MCU_SPEED}/1000000")

MESSAGE(STATUS "Built for ${LN_ARCH}/${LN_MCU} with ${LN_MCU_FLASH_SIZE} flash / ${LN_MCU_RAM_SIZE} ram")
MESSAGE(STATUS "          MCU running at ${MCU_SPEED_M} Mhz")


TUNE_NAME(TRUE ${MCU_SPEED_M}Mhz)
TUNE_NAME(USE_CLANG CLANG)
#TUNE_NAME(USE_RP2040_PURE_RAM RAM)


SET_TARGET_PROPERTIES(swindle PROPERTIES OUTPUT_NAME swindle${TGT_NAME})

MESSAGE(STATUS "Final name extension <${TGT_NAME}>")
#---
