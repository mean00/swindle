set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

cmake_minimum_required(VERSION 3.13)
message(STATUS "======================")
message(STATUS "Starting swindle ")
message(STATUS "======================")

option(SMALL_SWINDLE "Build a very limited small swindle" OFF)
option(USE_STLINK_PINOUT "Use SWD pins to control target" OFF)
option(USE_GD32F3 "Target GD32F303 chip with 256 flash" OFF)
option(USE_CH32V3x "Target USE_CH32V3x chip with 256 flash" OFF)
option(USE_RP_ZERO "Change RGB LED to map RP2040 board" OFF)
option(USE_RP_CARRIER "Use RP2040 carrier board" OFF)
option(USE_WS2812 "Use WS2812 RGB LED " OFF)
option(USE_RTT "Use RTT as logger " OFF)
option(USE_INVERTED_NRST "Inverted Reset signal (with a mosfet)" OFF)
option(USE_48PIN_PACKAGE "Use bluepill chip in 48 pins package (GD3/CH32)" OFF)
option(USE_64PIN_PACKAGE "Use bluepill chip in 64 pins package (GD3/CH32)" OFF)
option(SWINDLE_USE_USB "Use USB interface" ON)
option(SWINDLE_USE_NETWORK "Use tcp/ip interface" OFF)
option(LN_EXTERNAL_GENERATE "Build swindle as a subproject" OFF)
option(LN_DISABLE_SWINDLE_LNIO "Disable optimized IO path and revert to BMP ones" OFF)
option(SWINDLE_WITH_SDI "Build the WCH CH32V0xx single-wire (SDI) debug support" ON)
# Session-scoped static arena (malloc_used.md): the objects blackmagic allocates
# for a debug session come from two `.bss` regions instead of the FreeRTOS heap.
# Off => session_arena_stubs.rs forwards to the heap (old behaviour, no .bss).
option(SWINDLE_SESSION_ARENA "Serve session allocations from a static arena instead of the heap" ON)
# What those regions cost the probe, in KiB.  They are `.bss`
# (rs_swindle/src/session_arena_size.rs: POOL 1728 B + TRANSPORT 896 B = 2624 B,
# rounded up here), and LN_FREERTOS_HEAP_SIZE = RAM - LN_MCU_STATIC_RAM - 1, so
# the heap gives back exactly this much: see the block after
# include(./mcuSelect.cmake).
set(SWINDLE_SESSION_ARENA_RAM_KIB 3 CACHE STRING "KiB of static RAM taken by the session arena")
#
set(BMP_VERSION_BRANCH 1.10)
set(BMP_PATCH_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/patches_${BMP_VERSION_BRANCH})
set(LNBMP_TOP_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/blackmagic)
#
set(CMAKE_LINK_LIBRARIES_ONLY TRUE)
find_program(CCACHE_FOUND ccache)
if(CCACHE_FOUND)
  message(STATUS "Using CCACHE as ${CCACHE_FOUND}")
  set(CMAKE_C_COMPILER_LAUNCHER ${CCACHE_FOUND})
  set(CMAKE_CXX_COMPILER_LAUNCHER ${CCACHE_FOUND})
endif()
if(USE_RTT)
  add_definitions("-DUSE_RTT=1")
endif()
# The two big BMP tasks (gdbTask, usbserial) keep their TCB + stack in .bss on the
# boards that can afford the 7 KiB: RP2040/RP2350, the 39 KiB GD32F303 and the
# 64 KiB CH32V307.  A 20 KiB bluepill part keeps them on the heap.  The sizes are
# measured with `mon stacks`, see lnBmpTask.h and malloc_used.md §8.1 S5.
if(USE_GD32F3 OR USE_CH32V3x OR USE_RP2040 OR USE_RP2350)
  add_definitions("-DLN_BIG_TASKS_STATIC_STACK=1")
endif()

# -------------
# Options
# -------------
if(NOT USE_NO_DEFAULT)
  include(./build_default.cmake)
endif()
if(SMALL_WINDLE)
  add_definitions("-DSMALL_SWINDLE=1")
endif()
# Consistency check
if(NOT LN_EXTERNAL_GENERATE)
  if(SWINDLE_USE_USB AND SWINDLE_USE_NETWORK)
    message(FATAL_ERROR "Please Select usb or network")
  endif()
  if(NOT SWINDLE_USE_USB AND NOT SWINDLE_USE_NETWORK)
    message(FATAL_ERROR "Please Select usb or network")
  endif()
  if(SWINDLE_USE_NETWORK)
    if(USE_CH32V3x)
      set(LWIP_DIR
          ${CMAKE_CURRENT_SOURCE_DIR}/lwip
          CACHE INTERNAL "")
      set(LN_ENABLE_ETH
          TRUE
          CACHE INTERNAL "")
    endif()
  endif()
endif()
# ----------------
# Option loaded
# ----------------
# USB..
if(SWINDLE_USE_USB)
  if(USE_CH32V3x)
    set(USE_CH32v3x_USB_OTG
        True
        CACHE INTERNAL "")
  endif()
  # ___________________________________________________
  if(NOT LN_EXTERNAL_TUSBD)
    set(LN_ENABLE_USBD
        TRUE
        CACHE INTERNAL "")
  endif()
  # tiny USB configuration
  set(LN_USB_NB_HID 0) # No HID
  set(LN_USB_DFU_RUNTIME
      1
      CACHE INTERNAL "")

  set(LN_USB_BUFFER_SIZE
      256
      CACHE INTERNAL "") # increase usb cdc buffer size
else()
  set(LN_ENABLE_USBD
      FALSE
      CACHE INTERNAL "")
endif()
# /USB..
set(LN_ENABLE_I2C
    FALSE
    CACHE INTERNAL "")
set(LN_ENABLE_SPI
    FALSE
    CACHE INTERNAL "")
if(NOT LN_EXTERNAL_GENERATE)
  set(LN_ENABLE_UART
      TRUE
      CACHE INTERNAL "")
  set(LN_ENABLE_RUST
      TRUE
      CACHE INTERNAL "")
endif()

# ___________________________________________________
set(ESPRIT_ROOT ${CMAKE_CURRENT_SOURCE_DIR}/esprit)
include(./mcuSelect.cmake)
# The session arena's two regions are `.bss` (rs_swindle/src/session_arena_size.rs).
# lnFreeRTOS.cmake derives the heap from the board's static RAM
# (LN_FREERTOS_HEAP_SIZE = RAM - STATIC_RAM - 1), so give the arena its own budget
# there: the heap then gives up exactly the bytes the arena takes, and no board
# needs a hand-edited cache entry.
#
# mcuSelect.cmake only runs on the first configure (`if(NOT DEFINED LN_ARCH)`), so
# the board's own number is recorded once in LN_MCU_STATIC_RAM_BASE and the
# effective value is re-derived from it every time.  Deriving instead of adding in
# place is what keeps a repeated `cmake .` from growing the budget by one bump per
# configure.
#
# A build dir configured by an earlier revision of this feature carries the bump
# by hand in LN_MCU_STATIC_RAM; `cmake -U LN_MCU_STATIC_RAM .` does not undo that
# (the entry disappears and mcuSelect does not run again), so set the pristine
# board value once, e.g. `cmake -DLN_MCU_STATIC_RAM=31 .` for a GD32F303 that was
# configured with 34.
if(DEFINED LN_MCU_STATIC_RAM AND NOT DEFINED LN_MCU_STATIC_RAM_BASE)
  set(LN_MCU_STATIC_RAM_BASE
      ${LN_MCU_STATIC_RAM}
      CACHE INTERNAL "static RAM budget of the board, without the session arena")
endif()
if(SWINDLE_SESSION_ARENA AND DEFINED LN_MCU_STATIC_RAM_BASE)
  math(EXPR LN_MCU_STATIC_RAM "${LN_MCU_STATIC_RAM_BASE} + ${SWINDLE_SESSION_ARENA_RAM_KIB}")
  message(STATUS "Session arena: static RAM budget ${LN_MCU_STATIC_RAM} KiB (${LN_MCU_STATIC_RAM_BASE} + ${SWINDLE_SESSION_ARENA_RAM_KIB} KiB for the arena)")
endif()
set(CMAKE_TOOLCHAIN_FILE ${ESPRIT_ROOT}/esprit.cmake)
set(CMAKE_MODULE_PATH
    ${CMAKE_MODULE_PATH} ${CMAKE_CURRENT_SOURCE_DIR}/cmake ${ESPRIT_ROOT}/cmake
    CACHE INTERNAL "")

include(colorFormatting)
project(swindle NONE) # the NONE is very important !

enable_language(C CXX ASM) # this is important too!
# Generate version
string(TIMESTAMP timestamp "20%y-%m-%d")
include(swindle_version)
gen_version_file()

#
include(all_boards)
#
include(patch_blackmagic)

include_directories(${CMAKE_CURRENT_BINARY_DIR})
include_directories(${CMAKE_CURRENT_SOURCE_DIR})
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/setup/ch32v3xx)

if("${LN_USB_NB_CDC}" STREQUAL "3")
  message(STATUS "Enabling third CDC interface")
  add_definitions(-DUSE_3_CDC)
endif()

add_subdirectory(${ESPRIT_ROOT})

include(${ESPRIT_ROOT}/setup.cmake)

if(SWINDLE_USE_USB)
  if(NOT LN_EXTERNAL_TUSBD)
    use_library(tinyUsb)
    include(${ESPRIT_ROOT}/libraries/tinyUsb/tiny.cmake)
  endif()
endif()
# __________________________________________________________________
#
macro(TUNE_NAME OPTN NAME)
  if(${OPTN})
    string(APPEND TGT_NAME "_${NAME}")
    string(APPEND BUILD_STRING "${NAME}:")
  endif()
endmacro()
#

set(BMP ${CMAKE_CURRENT_SOURCE_DIR}/blackmagic)
#
if(LN_SWINDLE_AS_EXTERNAL)
  add_definitions("-DLN_SWINDLE_AS_EXTERNAL")
  # When built as an external library (e.g. the Rust wrapper), GENERATE_GD32_FIRMWARE
  # is not called, so create the aggregate `swindle` target here (placeholder source,
  # matching the old LN_EXTERNAL_GENERATE path). swindle_target.cmake builds the real
  # objects; the wrapper consumes them via $<TARGET_OBJECTS:swindle> etc.
  add_library(swindle STATIC src/swindle_empty.cpp)
else()

  if(USE_RTT)
    use_library(ln-rtt)
  endif()

  if(SWINDLE_USE_W5500)
    use_library(w5500)
  endif()

  #if(USE_WS2812)
  #use_library(WS2812B)
  #endif()
  add_subdirectory(src)
  set(SWINDLE_MAIN swindle_main )
  generate_gd32_firmware(swindle ${SOURCES})
  hash_gd32_firmware(swindle)
endif()

add_subdirectory(swindle)
target_link_libraries(swindle PUBLIC swindle_interface ${SWINDLE_MAIN})
# rsbmp_wrapper references symbols from swindleio_impl (via libswindle). Put rsbmp_wrapper before libswindle so the
# linker resolves its undefined symbols against the C++ platform library that follows.
target_link_libraries(swindle PUBLIC rsbmp_wrapper)
target_link_libraries(swindle PUBLIC libswindle)
# lnarduino_c_bindings is an OBJECT library; its objects are added to libswindle via swindle_target.cmake (see
# BRIDGE_SRCS). TARGET_LINK_LIBRARIES(swindle PUBLIC lnarduino_c_bindings) FIXME TODO this should be automatic
if(SWINDLE_USE_NETWORK)
  if(SWINDLE_USE_W5500)
    target_link_libraries(swindle_main PUBLIC w5500)
    target_include_directories(swindle PRIVATE ${CMAKE_SOURCE_DIR}/src/net ${CMAKE_SOURCE_DIR}/swindle/include ${CMAKE_SOURCE_DIR}/swindle/src ${CMAKE_SOURCE_DIR}/swindle/src/net)
    target_include_directories(swindle PRIVATE ${BMP}/src/include ${BMP}/src ${BMP}/src/target)
  else()
    target_link_libraries(swindle PUBLIC lwipcore lnEth_wch)
  endif()
endif()
#
#
#
if(NOT LN_EXTERNAL_GENERATE)
  #
  message(STATUS "=======")
  message(STATUS "Summary")
  message(STATUS "=======")

  math(EXPR MCU_SPEED_M "${LN_MCU_SPEED}/1000000")

  message(STATUS "Built for ${LN_ARCH}/${LN_MCU} with ${LN_MCU_FLASH_SIZE} flash / ${LN_MCU_RAM_SIZE} ram")
  message(STATUS "          MCU running at ${MCU_SPEED_M} Mhz")

  tune_name(USE_CH32V3x CH32V3x)
  tune_name(USE_GD32F3 GD32F3)
  tune_name(USE_RP2040 RP2040)
  tune_name(USE_RP2350 RP2350)
  tune_name(USE_RP_ZERO zero)
  tune_name(USE_64PIN_PACKAGE 64pins)
  tune_name(USE_48PIN_PACKAGE 48pins)
  tune_name(TRUE ${MCU_SPEED_M}Mhz)
  tune_name(USE_WS2812 WS2812)
  tune_name(USE_INVERTED_NRST INVRST)
  tune_name(USE_CLANG CLANG)
  tune_name(USE_RTT lnrtt)

  tune_name(SWINDLE_USE_USB usb)
  tune_name(SWINDLE_USE_NETWORK network)
  tune_name(SWINDLE_USE_W5500 W5500)
  tune_name(SMALL_SWINDLE bluepill)

  if(SWINDLE_USE_NETWORK)
    message(STATUS "   Ethernet version")
  elseif(SWINDLE_USE_USB)
    message(STATUS "   USB version")
  endif()

  # TUNE_NAME(USE_RP2040_PURE_RAM RAM)

  if(NOT LN_EXTERNAL_GENERATE)
    set_target_properties(swindle PROPERTIES OUTPUT_NAME swindle${TGT_NAME})
  endif()
  if(USE_WS2812)
    message(STATUS "   using WS2812B single led")
  endif()

  message(STATUS "Final name extension <${TGT_NAME}>")

  # get_target_property(if swindle INTERFACE_LINK_LIBRARIES) message(STATUS " *********** interface : <${if}>")

endif() # ---
file(WRITE ${CMAKE_BINARY_DIR}/swindle_build_options.h "#define SWINDLE_BUILD_OPTIONS \"${BUILD_STRING}\"")
