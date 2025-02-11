# ===========================================================================================
#
#
#
#
#
# ===========================================================================================
MESSAGE(STATUS "Building for embedded mode (${SWINDLE_HOSTED})")
IF(USE_RP2040)
  SET(EXTRA _rp2040)
ELSE()
  SET(EXTRA _ln)
ENDIF()
SET(BRIDGE_SRCS
                ${B}/bridge.cpp
                ${B}/bmp_gpio.cpp
                ${B}/bmp_adc${EXTRA}.cpp
                ${B}/bmp_serial.cpp
                ${B}/bmp_rs_gdb.cpp
                ${B}/bmp_jtagstubs.cpp
                ${B}/bmp_tap.cpp
                CACHE INTERNAL ""
                )
# #
include(./swindle_common.cmake)
# ===========================================================================================
include_directories( ${CMAKE_CURRENT_SOURCE_DIR}/include)
include_directories( ${BMP}/src)
include_directories( ${BMP}/src/include)
include_directories( ${BMP}/src/target)
# ===========================================================================================
ADD_DEFINITIONS("-DENABLE_DEBUG=1")
ADD_DEFINITIONS("-DPLATFORM_IDENT=\"lnBMP\"")
ADD_DEFINITIONS("-DPC_HOSTED=0")
ADD_DEFINITIONS("-include miniplatform.h")
ADD_DEFINITIONS("-DBMD_IS_STDC=1")
# ===========================================================================================
#  add swindle target code and host connection stuff
IF(USE_RP2040)
  IF(TRUE)
    RP_PIO_GENERATE( ${CMAKE_CURRENT_SOURCE_DIR}/src/swd.pio ${CMAKE_BINARY_DIR}/bmp_pio_swd.h)
    RP_PIO_GENERATE( ${CMAKE_CURRENT_SOURCE_DIR}/src/rvswd.pio ${CMAKE_BINARY_DIR}/bmp_pio_rvswd.h)
    SET( EXTRA_SOURCE ${CMAKE_BINARY_DIR}/bmp_pio_swd.h ${CMAKE_BINARY_DIR}/bmp_pio_rvswd.h ${B}/bmp_rvTap${EXTRA}.cpp   ${B}/bmp_swdTap${EXTRA}.cpp ${B}/bmp_tap_rp2040.cpp  )
  ELSE()
    SET( EXTRA_SOURCE  ${B}/bmp_rvTap.cpp   ${B}/bmp_swdTap.cpp  ${B}/bmp_tap_gpio.cpp )
  ENDIF()
  IF(USE_RP_CARRIER)
    MESSAGE(STATUS "RP2040 using carrier board layout")
    ADD_DEFINITIONS("-DUSE_RP_CARRIER")
  ENDIF()

ELSE()
  SET( EXTRA_SOURCE  ${B}/bmp_rvTap.cpp   ${B}/bmp_swdTap.cpp  ${B}/bmp_tap_gpio.cpp )
  #set_property(SOURCE src/bmp_rvTap.cpp  PROPERTY COMPILE_OPTIONS "-Os")
ENDIF()
# O0, 1, 2 works
# OZ does not work
# rvTap does not like -Oz
# Something to fix here
#MESSAGE(STATUS "Restricting flags for rvTap to -Os")
#
# ===========================================================================================
ADD_SUBDIRECTORY(${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle/c_interface bmp_c_interface)
ADD_SUBDIRECTORY( rs )
# ===========================================================================================

ADD_LIBRARY(libswindle STATIC ${BM_SRC} ${BRIDGE_SRCS}  ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE} )
TARGET_INCLUDE_DIRECTORIES( libswindle PRIVATE ${BMP_EXTRA} ${CMAKE_CURRENT_SOURCE_DIR}/include)
TARGET_INCLUDE_DIRECTORIES( libswindle PRIVATE ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config )
TARGET_INCLUDE_DIRECTORIES( libswindle PRIVATE ${myB}/private_include)
TARGET_INCLUDE_DIRECTORIES( libswindle PUBLIC  ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS} )
TARGET_LINK_LIBRARIES( libswindle lnArduino tinyUsb)
IF(USE_GD32F3)
  TARGET_COMPILE_DEFINITIONS( libswindle PUBLIC  USE_GD32F303)
ENDIF()
IF(USE_RP2040)
  TARGET_LINK_LIBRARIES( libswindle rplib )
ENDIF()

#---------
#
#
# ===========================================================================================
