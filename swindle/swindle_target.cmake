# ===========================================================================================
#
#
#
#
#
# ===========================================================================================
MESSAGE(STATUS "Building for embedded mode (${SWINDLE_HOSTED})")
IF(USE_RP2040 OR USE_RP2350)
  SET(EXTRA _rp2040)
ELSE()
  SET(EXTRA _ln)
ENDIF()
SET(BRIDGE_SRCS
                ${B}/bridge.cpp
                ${B}/bmp_gpio.cpp
                ${B}/bmp_jtagstubs.cpp
                ${B}/bmp_tap.cpp
                CACHE INTERNAL ""
                )
IF(SWINDLE_USE_USB)
  IF(NOT LN_SWINDLE_AS_EXTERNAL)
    LIST( APPEND BRIDGE_SRCS ${B}/bmp_serial.cpp
                ${B}/bmp_usb.cpp
  )
    IF("${LN_USB_NB_CDC}" STREQUAL "3")
      SET(BRIDGE_SRCS ${BRIDGE_SRCS} ${B}/bmp_cdc_logger.cpp)
    ENDIF()
  ENDIF()
ENDIF()
IF(SWINDLE_USE_NETWORK)
  LIST( APPEND BRIDGE_SRCS    ${B}/bmp_net.cpp)
ENDIF()



# #
include(./swindle_common.cmake)
# ==========================================================================
include_directories( ${CMAKE_CURRENT_SOURCE_DIR}/include)
include_directories( ${BMP}/src)
include_directories( ${BMP}/src/include)
include_directories( ${BMP}/src/target)
# ==========================================================================
ADD_DEFINITIONS("-DENABLE_DEBUG=1")
ADD_DEFINITIONS("-DPLATFORM_IDENT=\"lnBMP\"")
ADD_DEFINITIONS("-DPC_HOSTED=0")
ADD_DEFINITIONS("-include miniplatform.h")
ADD_DEFINITIONS("-DBMD_IS_STDC=1")
# ==========================================================================
if(USE_INVERTED_NRST )
  SET( EXTRA_SOURCE  ${EXTRA_SOURCE} ${B}/bmp_reset_inv.cpp)
else()
  SET( EXTRA_SOURCE  ${EXTRA_SOURCE} ${B}/bmp_reset.cpp)
endif()

# O0, 1, 2 works
# OZ does not work
# rvTap does not like -Oz
# Something to fix here
#MESSAGE(STATUS "Restricting flags for rvTap to -Os")
#
# ===========================================================================================
ADD_SUBDIRECTORY(${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle/c_interface bmp_c_interface)
IF(LN_EXTERNAL_RUST)
ELSE()
  ADD_SUBDIRECTORY( rs )
ENDIF()
# ===========================================================================================

ADD_LIBRARY(libswindle STATIC ${BM_SRC} ${BRIDGE_SRCS}  ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE} )
TARGET_INCLUDE_DIRECTORIES( libswindle PRIVATE ${BMP_EXTRA} ${CMAKE_CURRENT_SOURCE_DIR}/include)
TARGET_INCLUDE_DIRECTORIES( libswindle PRIVATE ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config )
TARGET_INCLUDE_DIRECTORIES( libswindle PRIVATE ${myB}/private_include)
TARGET_INCLUDE_DIRECTORIES( libswindle PUBLIC  ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS} )
TARGET_LINK_LIBRARIES( libswindle PUBLIC esprit_dev )
#
#
#
IF(USE_RP2040 OR USE_RP2350)
  include_directories(src/rp2040)
  add_subdirectory(src/rp2040)
ELSEIF("${LN_MCU}" STREQUAL "ESP32")
  include_directories(src/esp32_gpio)
  add_subdirectory(src/esp32_gpio)
ELSE()
  include_directories(src/ln)
  add_subdirectory(src/ln)
ENDIF()

TARGET_LINK_LIBRARIES( libswindle PUBLIC swindleio_impl )
IF(USE_GD32F3)
  TARGET_COMPILE_DEFINITIONS( libswindle PUBLIC  USE_GD32F303)
ENDIF()

#---------
#
#
# ===========================================================================================
