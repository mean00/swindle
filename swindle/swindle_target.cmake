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
if(SWINDLE_USE_USB)
  if(NOT LN_SWINDLE_AS_EXTERNAL)
    list(APPEND BRIDGE_SRCS # ${B}/bmp_serial.cpp
         ${B}/usb/bmp_usb.cpp ${ESPRIT_ROOT}/rust/rust_esprit/c_interface/lnSerial_c.cpp)
    # IF("${LN_USB_NB_CDC}" STREQUAL "3") SET(BRIDGE_SRCS ${BRIDGE_SRCS} ${B}/bmp_cdc_logger.cpp) ENDIF()
  endif()
endif()
if(SWINDLE_USE_NETWORK)
  list(APPEND BRIDGE_SRCS ${B}/net/bmp_net.cpp)
  include_directories(${CMAKE_CURRENT_SOURCE}/net)
endif()

# #
include(./swindle_common.cmake)
# ==========================================================================
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)
include_directories(${BMP}/src)
include_directories(${BMP}/src/include)
include_directories(${BMP}/src/target)
# ==========================================================================
add_definitions("-DENABLE_DEBUG=1")
add_definitions("-DPLATFORM_IDENT=\"lnBMP\"")
add_definitions("-DPC_HOSTED=0")
add_definitions("-include miniplatform.h")
add_definitions("-DBMD_IS_STDC=1")
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
target_include_directories(libswindle PRIVATE ${BMP_EXTRA} ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_include_directories(libswindle PRIVATE ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config)
target_include_directories(libswindle PRIVATE ${myB}/private_include)
target_include_directories(libswindle PUBLIC ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS})
target_link_libraries(libswindle PUBLIC esprit_dev)
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
if(USE_GD32F3)
  target_compile_definitions(libswindle PUBLIC USE_GD32F303)
endif()

# ---------
#
# ===========================================================================================
