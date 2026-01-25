
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
OPTION(USE_WS2812        "Use WS2812 RGB LED " OFF)
OPTION(USE_INVERTED_NRST    "Inverted Reset signal (with a mosfet)" OFF)
OPTION(USE_48PIN_PACKAGE "Use bluepill chip in 48 pins package (GD3/CH32)" OFF)
OPTION(USE_64PIN_PACKAGE "Use bluepill chip in 64 pins package (GD3/CH32)" OFF)
OPTION(SWINDLE_USE_USB "Use USB interface" ON)
OPTION(SWINDLE_USE_NETWORK "Use tcp/ip interface" OFF)
OPTION(LN_EXTERNAL_GENERATE "Build swindle as a subproject" OFF)
OPTION(LN_DISABLE_SWINDLE_LNIO "Disable optimized IO path and revert to BMP ones" OFF)
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
# Consistency check
IF(NOT LN_EXTERNAL_GENERATE)
  if(SWINDLE_USE_USB AND SWINDLE_USE_NETWORK)
    MESSAGE(FATAL_ERROR "Please Select usb or network")
  endif()
  if(NOT SWINDLE_USE_USB AND NOT SWINDLE_USE_NETWORK)
    MESSAGE(FATAL_ERROR "Please Select usb or network")
  endif()
  if(SWINDLE_USE_NETWORK)
    set(LWIP_DIR ${CMAKE_CURRENT_SOURCE_DIR}/lwip CACHE INTERNAL "")
    set(LN_ENABLE_ETH TRUE CACHE INTERNAL "")
  endif()
ENDIF()
#----------------
# Option loaded
#----------------
IF(USE_WS2812)
  SET(EXTRA_WS2812 "_ws2812")
ENDIF()
# USB..
IF(SWINDLE_USE_USB)
  IF(USE_CH32V3x)
    set(USE_CH32v3x_USB_OTG True CACHE INTERNAL "")
  ENDIF()
  #___________________________________________________
  IF(NOT LN_EXTERNAL_TUSBD)
    set(LN_ENABLE_USBD  TRUE  CACHE INTERNAL "")
  ENDIF()
  # tiny USB configuration
  set(LN_USB_NB_HID 0) # No HID
  set(LN_USB_DFU_RUNTIME 1 CACHE INTERNAL "")

  set(LN_USB_BUFFER_SIZE 256 CACHE INTERNAL "") # increase usb cdc buffer size
ELSE()
  set(LN_ENABLE_USBD  FALSE  CACHE INTERNAL "")
ENDIF()
# /USB..
set(LN_ENABLE_I2C   FALSE CACHE INTERNAL "")
set(LN_ENABLE_SPI   FALSE CACHE INTERNAL "")
IF(NOT LN_EXTERNAL_GENERATE)
  set(LN_ENABLE_UART  TRUE  CACHE INTERNAL "")
  set(LN_ENABLE_RUST  TRUE  CACHE INTERNAL "")
ENDIF()

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
INCLUDE_DIRECTORIES(${CMAKE_CURRENT_SOURCE_DIR}/setup/ch32v3xx)

IF("${LN_USB_NB_CDC}" STREQUAL "3")
  MESSAGE(STATUS "Enabling third CDC interface")
  ADD_DEFINITIONS(-DUSE_3_CDC)
ENDIF()

ADD_SUBDIRECTORY(${ESPRIT_ROOT})

INCLUDE(${ESPRIT_ROOT}/setup.cmake)

IF(SWINDLE_USE_USB)
  IF(NOT LN_EXTERNAL_TUSBD)
    USE_LIBRARY(tinyUsb)
    INCLUDE(${ESPRIT_ROOT}/libraries/tinyUsb/tiny.cmake)
  ENDIF()
ENDIF()
#__________________________________________________________________
#
#
#
MACRO(TUNE_NAME OPTN NAME)
  IF(${OPTN})
    STRING(APPEND TGT_NAME "_${NAME}")
    STRING(APPEND BUILD_STRING "${NAME}:")
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
    set(SOURCES src/swindle${EXTRA_WS2812}.cpp)
  ENDIF()
ENDIF()

IF(NOT LN_EXTERNAL_GENERATE)
  IF(USE_WS2812)
    USE_LIBRARY(WS2812B)
  ENDIF()
  GENERATE_GD32_FIRMWARE(swindle ${SOURCES})
  HASH_GD32_FIRMWARE(swindle)
ELSE()
  ADD_LIBRARY(swindle STATIC src/swindle_empty.cpp)
ENDIF()

ADD_SUBDIRECTORY(swindle)
TARGET_LINK_LIBRARIES(swindle PUBLIC libswindle)
TARGET_LINK_LIBRARIES(swindle PUBLIC swindle_interface)
TARGET_LINK_LIBRARIES(swindle PUBLIC rsbmp)
#FIXME TODO this should be automatic
if(SWINDLE_USE_NETWORK)
  TARGET_LINK_LIBRARIES(swindle PUBLIC lwipcore lnEth_wch)
endif()
#
TUNE_NAME(USE_CH32V3x CH32V3x)
TUNE_NAME(USE_GD32F3  GD32F3)
TUNE_NAME(USE_RP2040  RP2040)
TUNE_NAME(USE_RP2350  RP2350)
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
  TUNE_NAME(USE_WS2812  WS2812)
  TUNE_NAME(USE_INVERTED_NRST  INVRST)
  TUNE_NAME(USE_CLANG CLANG)
  TUNE_NAME(USE_64PIN_PACKAGE 64pins)
  TUNE_NAME(USE_48PIN_PACKAGE 48pins)

  TUNE_NAME(SWINDLE_USE_USB  usb)
  TUNE_NAME(SWINDLE_USE_NETWORK  network)

  if(SWINDLE_USE_NETWORK)
    message(STATUS "   Ethernet version")
  elseif(SWINDLE_USE_USB)
    message(STATUS "   USB version")
  endif()

  #TUNE_NAME(USE_RP2040_PURE_RAM RAM)

  IF(NOT LN_EXTERNAL_GENERATE)
    SET_TARGET_PROPERTIES(swindle PROPERTIES OUTPUT_NAME swindle${TGT_NAME})
  ENDIF()
  IF(USE_WS2812)
    MESSAGE(STATUS "   using WS2812B single led")
  ENDIF()

  MESSAGE(STATUS "Final name extension <${TGT_NAME}>")

  #get_target_property(if swindle INTERFACE_LINK_LIBRARIES)
  #message(STATUS " *********** interface : <${if}>")

ENDIF() #---
FILE(WRITE ${CMAKE_BINARY_DIR}/swindle_build_options.h "#define SWINDLE_BUILD_OPTIONS \"${BUILD_STRING}\"")

