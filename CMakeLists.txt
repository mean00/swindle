
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

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
set(BMP_VERSION_BRANCH 1.10)
set(BMP_PATCH_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/patches_${BMP_VERSION_BRANCH})
set(LNBMP_TOP_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/blackmagic)
#
#
set(CMAKE_LINK_LIBRARIES_ONLY TRUE)
find_program(CCACHE_FOUND ccache)
if(CCACHE_FOUND)
  MESSAGE(STATUS "Using CCACHE as ${CCACHE_FOUND}")
  set(CMAKE_C_COMPILER_LAUNCHER ${CCACHE_FOUND})
  set(CMAKE_CXX_COMPILER_LAUNCHER ${CCACHE_FOUND})
endif()


#-------------
# Options
#-------------
IF(NOT USE_NO_DEFAULT)
  INCLUDE(./build_default.cmake)
ENDIF()

IF(USE_CH32V3x)
  set(USE_CH32v3x_USB_OTG True CACHE INTERNAL "")
ENDIF()
#___________________________________________________
IF(NOT LN_EXTERNAL_TUSBD)
  set(LN_ENABLE_USBD  TRUE  CACHE INTERNAL "")
ENDIF()
set(LN_ENABLE_I2C   FALSE CACHE INTERNAL "")
set(LN_ENABLE_SPI   FALSE CACHE INTERNAL "")
set(LN_ENABLE_UART  TRUE  CACHE INTERNAL "")
set(LN_ENABLE_RUST  TRUE  CACHE INTERNAL "")

# tiny USB configuration
set(LN_USB_NB_HID 0) # No HID
set(LN_USB_DFU_RUNTIME 1 CACHE INTERNAL "")

set(LN_USB_BUFFER_SIZE 256 CACHE INTERNAL "") # increase usb cdc buffer size
#___________________________________________________

#set(USE_RP2040_PURE_RAM True CACHE INTERNAL "")
#___________________________________________________
set(ESPRIT_ROOT  ${CMAKE_CURRENT_SOURCE_DIR}/esprit)
INCLUDE(./mcuSelect.cmake)
set(CMAKE_TOOLCHAIN_FILE ${ESPRIT_ROOT}/esprit.cmake)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${CMAKE_CURRENT_SOURCE_DIR}/cmake ${ESPRIT_ROOT}/cmake CACHE INTERNAL "")

include(colorFormatting)
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

INCLUDE_DIRECTORIES(${CMAKE_CURRENT_BINARY_DIR})
INCLUDE_DIRECTORIES(${CMAKE_CURRENT_SOURCE_DIR})
INCLUDE_DIRECTORIES(${CMAKE_CURRENT_SOURCE_DIR}/include)

IF("${LN_USB_NB_CDC}" STREQUAL "3")
  MESSAGE(STATUS "Enabling third CDC interface")
  ADD_DEFINITIONS(-DUSE_3_CDC)
ENDIF()

ADD_SUBDIRECTORY(${ESPRIT_ROOT})

INCLUDE(${ESPRIT_ROOT}/setup.cmake)

IF(NOT LN_EXTERNAL_TUSBD)
  USE_LIBRARY(tinyUsb)
  INCLUDE(${ESPRIT_ROOT}/libraries/tinyUsb/tiny.cmake)
ENDIF()
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


set(BMP ${CMAKE_CURRENT_SOURCE_DIR}/blackmagic)
IF(LN_SWINDLE_AS_EXTERNAL)
  ADD_DEFINITIONS("-DLN_SWINDLE_AS_EXTERNAL")
ELSE()

  IF(USE_RP2040 OR USE_RP2350)
    RP_PIO_GENERATE( ${CMAKE_CURRENT_SOURCE_DIR}/src/ws.pio ${CMAKE_BINARY_DIR}/ws.h)
    IF(USE_RP_ZERO)
      TUNE_NAME(USE_RP_ZERO zero)
      set(ZERO_FLAGS _zero)
    ENDIF()
    IF(USE_RP_CARRIER)
      TUNE_NAME(USE_RP_CARRIER carrier)
    ENDIF()
    set(SOURCES src/swindle_rp2040${ZERO_FLAGS}.cpp ${CMAKE_BINARY_DIR}/ws.h)
  ELSE()
    set(SOURCES src/swindle.cpp)
  ENDIF()
ENDIF()

IF(NOT LN_EXTERNAL_GENERATE)
  GENERATE_GD32_FIRMWARE(swindle ${SOURCES})
  HASH_GD32_FIRMWARE(swindle)
ELSE()
  ADD_LIBRARY(swindle STATIC src/swindle_empty.cpp)
ENDIF()

ADD_SUBDIRECTORY(swindle)
TARGET_LINK_LIBRARIES(swindle PUBLIC libswindle)
TARGET_LINK_LIBRARIES(swindle PUBLIC swindle_interface)
TARGET_LINK_LIBRARIES(swindle PUBLIC rsbmp)

#
TUNE_NAME(USE_CH32V3x CH32V3x)
TUNE_NAME(USE_GD32F3  GD32F3)
TUNE_NAME(USE_RP2040  RP2040)
TUNE_NAME(USE_RP2350  RP2350)
#
#
IF(NOT LN_EXTERNAL_GENERATE)
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

  #get_target_property(if swindle INTERFACE_LINK_LIBRARIES)
  #message(STATUS " *********** interface : <${if}>")



ENDIF() #---
