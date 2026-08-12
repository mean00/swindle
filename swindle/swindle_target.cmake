# ===========================================================================================
#
# ===========================================================================================
message(STATUS "Building for embedded mode (${SWINDLE_HOSTED})")
if(USE_RP2040 OR USE_RP2350)
  set(EXTRA _rp2040)
else()
  set(EXTRA _ln)
endif()
set(BRIDGE_SRCS
      ${B}/bridge.cpp ${B}/bmp_gpio.cpp ${B}/bmp_jtagstubs.cpp ${B}/bmp_reset_pin.cpp
      CACHE INTERNAL "")
if(SWINDLE_USE_W5500)
  # W5500 builds provide their own user_init() and don't need bridge.cpp
  # Also include lnSocketRunner.cpp directly (not compiled via ln_utils when LN_ENABLE_ETH=OFF)
  set(BRIDGE_SRCS ${BRIDGE_SRCS}   ${ESPRIT_ROOT}/src/lnSocketRunner.cpp CACHE INTERNAL "")
endif()
if(SWINDLE_USE_USB)
  if(NOT LN_SWINDLE_AS_EXTERNAL)
    list(APPEND BRIDGE_SRCS # ${B}/bmp_serial.cpp
         ${B}/usb/bmp_usb.cpp ${ESPRIT_ROOT}/rust/rust_esprit/c_interface/lnSerial_c.cpp)
    # IF("${LN_USB_NB_CDC}" STREQUAL "3") SET(BRIDGE_SRCS ${BRIDGE_SRCS} ${B}/bmp_cdc_logger.cpp) ENDIF()
  endif()
endif()
if(SWINDLE_USE_NETWORK)
  if(USE_RP2040 OR USE_RP2350)
    # RP2040 + W5500: include bmp_net.cpp for socketRunner implementation
    include_directories(${CMAKE_SOURCE_DIR}/src/net)
    list(APPEND BRIDGE_SRCS ${B}/net/bmp_net.cpp)
  else()
    list(APPEND BRIDGE_SRCS ${B}/net/bmp_net.cpp)
    include_directories(${CMAKE_CURRENT_SOURCE}/net)
  endif()
endif()

# #
# bmp_core (defined in swindle_common.cmake) now carries the blackmagic compile
# definitions + include folders that used to be pushed into the whole swindle
# directory here. Each target compiling blackmagic code links it explicitly.
include(./swindle_common.cmake)
# ==========================================================================
if(USE_INVERTED_NRST)
  set(EXTRA_SOURCE ${EXTRA_SOURCE} ${B}/bmp_reset_inv.cpp)
else()
  set(EXTRA_SOURCE ${EXTRA_SOURCE} ${B}/bmp_reset.cpp)
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

add_library(libswindle STATIC ${BM_SRC} ${BRIDGE_SRCS} ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE})
# blackmagic_addon (${BMP_EXTRA}) is a libswindle-only include (the addon
# sources compile inside libswindle); the rest of the blackmagic context
# (${BMP}/src* dirs + defines) comes from bmp_core (linked below).
target_include_directories(libswindle PRIVATE ${BMP_EXTRA})
target_include_directories(libswindle PRIVATE ${B}/include ${CMAKE_BINARY_DIR}/config)
target_include_directories(libswindle PRIVATE ${myB}/private_include)
target_include_directories(libswindle PUBLIC ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS})
target_link_libraries(libswindle PUBLIC esprit_dev)
target_link_libraries(libswindle PRIVATE bmp_core)
# Force-include the generated platformgenerated.h (target-own option, see
# swindle_common.cmake).
ln_force_platformgen(libswindle)
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

target_link_libraries(libswindle PUBLIC swindleio_impl)
# Board-selection defines (USE_RP_CARRIER, LN_UART_*, USE_48PIN_PACKAGE ...)
# live on the ln_bsp interface, populated by the platform CMakeLists above.
target_link_libraries(libswindle PUBLIC ln_bsp)
if(USE_GD32F3)
  target_compile_definitions(libswindle PUBLIC USE_GD32F303)
endif()

# ---------
#
# ===========================================================================================
