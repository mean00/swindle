# ===========================================================================================
#
# ===========================================================================================
message(STATUS "Building for embedded mode (${SWINDLE_HOSTED})")
if(USE_RP2040 OR USE_RP2350)
  set(EXTRA _rp2040)
else()
  set(EXTRA _ln)
endif()
# bridge.cpp carries the platform glue plus the two symbols that used to have a file
# of their own: `platform_target_clk_output_enable()` (was bmp_gpio.cpp, a no-op every
# platform shares) and, through the taps, the `swd_frequency` accessor. bmp_gpio.cpp's
# other two functions (bmp_gpio_write/read) were `xAssert(0)` with no caller.
# bmp_reset_pin.cpp is no longer needed where the tap owns `swd_frequency`
# (LN + RP2040/RP2350 define `bmp_get_frequency_c()` themselves); the ESP32 taps
# still rely on it, so it stays an ESP32-only entry below.
set(BRIDGE_SRCS "" CACHE INTERNAL "")
if(NOT LN_ENABLE_RUST)
  list(APPEND BRIDGE_SRCS ${B}/bridge.cpp)
endif()
# Target nRST: driven in Rust on every board that has a table
# (rs_swindle_board's ln::/rp2xxx::reset_pin — GD32/CH32 and RP2040/RP2350). The
# ESP32 is the last platform that still goes through the C++ SwdReset (pReset), and
# that is the only build that may define these symbols and the class. Its taps
# also keep using bmp_reset_pin.cpp's bmp_get_frequency_c().
if("${LN_MCU}" STREQUAL "ESP32")
  list(APPEND BRIDGE_SRCS ${B}/bmp_nrst_c.cpp ${B}/bmp_reset.cpp ${B}/bmp_reset_inv.cpp
       ${B}/bmp_reset_pin.cpp)
endif()
if(SWINDLE_USE_W5500)
  # W5500 builds provide their own user_init() and don't need bridge.cpp
endif()
if(SWINDLE_USE_USB)
  if(NOT LN_SWINDLE_AS_EXTERNAL)
    # lnSerial_c.cpp is NOT listed here: it is the C ABI of the C++ UART drivers, which
    # a Rust build does not link (esprit/rust/CMakeLists.txt adds it only when
    # LN_ENABLE_RUST is off). Listing it here too compiled the same TU twice.
    if(NOT (LN_ENABLE_RUST AND LN_RUST_USB AND ("${LN_EXT}" STREQUAL "arm_gd32fx" OR "${LN_EXT}" STREQUAL "riscv_ch32v3x")))
      list(APPEND BRIDGE_SRCS # ${B}/bmp_serial.cpp
           ${B}/usb/bmp_usb.cpp)
    endif()
    # IF("${LN_USB_NB_CDC}" STREQUAL "3") SET(BRIDGE_SRCS ${BRIDGE_SRCS} ${B}/bmp_cdc_logger.cpp) ENDIF()
  endif()
endif()
if(SWINDLE_USE_NETWORK)
  # Socket runner is now implemented in Rust (rs_socketrunner + rs_swindle::net)
  include_directories(${CMAKE_CURRENT_SOURCE}/net)
endif()

# #
# bmp_core (defined in swindle_common.cmake) now carries the blackmagic compile
# definitions + include folders that used to be pushed into the whole swindle
# directory here. Each target compiling blackmagic code links it explicitly.
include(./swindle_common.cmake)
# ==========================================================================
if("${LN_MCU}" STREQUAL "ESP32")
  # The C++ reset driver (SwdReset / pReset): only the ESP32 taps still use it. The
  # GD32/CH32 and RP2040/RP2350 boards drive nRST from their board table in Rust
  # (rs_swindle_board's ln::/rp2xxx::reset_pin).
  if(USE_INVERTED_NRST)
    set(EXTRA_SOURCE ${EXTRA_SOURCE} ${B}/bmp_reset_inv.cpp)
  else()
    set(EXTRA_SOURCE ${EXTRA_SOURCE} ${B}/bmp_reset.cpp)
  endif()
endif()

# O0, 1, 2 works OZ does not work rvTap does not like -Oz Something to fix here MESSAGE(STATUS "Restricting flags for
# rvTap to -Os")
#
# ===========================================================================================
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle/c_interface bmp_c_interface)
if(LN_EXTERNAL_RUST)

else()
  add_subdirectory(rs)
endif()
# ===========================================================================================

set(_libswindle_srcs ${BM_SRC} ${BRIDGE_SRCS} ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE})
if(_libswindle_srcs)
  add_library(libswindle STATIC ${_libswindle_srcs})
  if(TARGET swindle_board_header)
    add_dependencies(libswindle swindle_board_header)
  endif()
else()
  add_library(libswindle INTERFACE)
endif()
# ---------------------------------------------------------------------------
# Session arena (malloc_used.md): force-include the allocation shim into the
# blackmagic C sources only, so that the `calloc`/`malloc`/`free` of the
# ADIv5/Cortex/RISC-V layer hit rs_swindle's static arena instead of the heap.
# Scoped to .c files on purpose: esprit/include/esprit.h declares malloc/free
# again (with _NOTHROW / __attribute_malloc__) and lnRingBuffer.h has a member
# function named free(), so the macros must never be active in a C++ TU that
# sees those headers.  bmp_helpers.cpp (the other holder of session allocations)
# is C++ too, so it includes only the declarations (swindle/include/session_arena.h)
# and names the swindle_session_* functions explicitly.  With
# SWINDLE_SESSION_ARENA=OFF this block is skipped and the firmware is exactly the
# pre-arena one.
# ---------------------------------------------------------------------------
if(SWINDLE_SESSION_ARENA)
  set(_arena_shim ${myB}include/session_arena_shim.h)
  set(_arena_c_sources "")
  foreach(_src IN LISTS BOARDS BM_TARGET)
    if(_src MATCHES "\\.c$")
      list(APPEND _arena_c_sources ${_src})
    endif()
  endforeach()
  if(_arena_c_sources AND EXISTS ${_arena_shim})
    set_property(SOURCE ${_arena_c_sources} APPEND PROPERTY
                 COMPILE_OPTIONS "-include" "${_arena_shim}")
    message(STATUS "Session arena: allocation shim force-included into ${_arena_c_sources}")
  else()
    message(WARNING "Session arena: no C sources / no shim at ${_arena_shim}")
  endif()
endif()
# blackmagic_addon (${BMP_EXTRA}) is a libswindle-only include (the addon
# sources compile inside libswindle); the rest of the blackmagic context
# (${BMP}/src* dirs + defines) comes from bmp_core (linked below).
if(_libswindle_srcs)
  target_include_directories(libswindle PRIVATE ${BMP_EXTRA})
  target_include_directories(libswindle PRIVATE ${B}/include ${CMAKE_BINARY_DIR}/config)
  target_include_directories(libswindle PRIVATE ${myB}/private_include)
  target_include_directories(libswindle PUBLIC ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS})
  target_link_libraries(libswindle PUBLIC esprit_dev)
  target_link_libraries(libswindle PRIVATE bmp_core)
  # Force-include the generated platformgenerated.h (target-own option, see
  # swindle_common.cmake).
  ln_force_platformgen(libswindle)
else()
  target_include_directories(libswindle INTERFACE ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS})
  target_link_libraries(libswindle INTERFACE esprit_dev)
endif()
#
if(USE_RP2040 OR USE_RP2350)
  include_directories(src/platform/rp2040)
  add_subdirectory(src/platform/rp2040)
elseif("${LN_MCU}" STREQUAL "ESP32")
  set(ESP32_IMPL esp32_fastgpio)
  # set(ESP32_IMPL esp32_gpio) set(ESP32_IMPL esp32_spi)
  include_directories(src/platform/esp32_pinout)
  include_directories(src/platform/${ESP32_IMPL})
  add_subdirectory(src/platform/${ESP32_IMPL})
  if("${LN_ESP_BOARD}" STREQUAL "mini")
    target_compile_definitions(swindleio_impl PRIVATE "-DLN_ESP_MINI=1")
    message(STATUS "Using ESP32S3 Mini pinout")
  else()
    target_compile_definitions(swindleio_impl PRIVATE "-DLN_ESP_WROOM=1")
    message(STATUS "Using ESP32S3 WROOM pinout")
  endif()
else()
  include_directories(src/platform/ln)
  add_subdirectory(src/platform/ln)
endif()

if(_libswindle_srcs)
  target_link_libraries(libswindle PUBLIC swindleio_impl)
  target_link_libraries(libswindle PUBLIC ln_bsp)
  if(USE_GD32F3)
    target_compile_definitions(libswindle PUBLIC USE_GD32F303)
  endif()
else()
  target_link_libraries(libswindle INTERFACE swindleio_impl ln_bsp)
  if(USE_GD32F3)
    target_compile_definitions(libswindle INTERFACE USE_GD32F303)
  endif()
endif()

# ---------
#
# ===========================================================================================
