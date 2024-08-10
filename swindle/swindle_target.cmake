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
SET(BMP_EXTRA  ${B}/../../blackmagic_addon/)
SET(BOARDS      ${enabled_sources}
                ${BMP_EXTRA}/target/ch32v3xx.c
                ${B}/bmp_disabledBoard.cpp
                ${T}/lpc_common.c
                CACHE INTERNAL ""
        )
#
SET(BRIDGE_SRCS
                ${B}/bridge.cpp
                ${B}/bmp_gpio.cpp
                ${B}/bmp_adc${EXTRA}.cpp
                ${B}/bmp_serial.cpp
                ${B}/bmp_rs_gdb.cpp
                ${B}/bmp_jtagstubs.cpp
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
    SET( EXTRA_SOURCE ${CMAKE_BINARY_DIR}/bmp_pio_swd.h ${B}/bmp_rvTap${EXTRA}.cpp   ${B}/bmp_swdTap${EXTRA}.cpp   )
  ELSE()
    SET( EXTRA_SOURCE  ${B}/bmp_rvTap.cpp   ${B}/bmp_swdTap.cpp   )
  ENDIF()
ELSE()
  SET( EXTRA_SOURCE  ${B}/bmp_rvTap.cpp   ${B}/bmp_swdTap.cpp   )
ENDIF()
#
# ===========================================================================================
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle/c_interface bmp_c_interface)
add_subdirectory( rs )
# ===========================================================================================

add_library(libswindle STATIC ${BM_SRC} ${BRIDGE_SRCS}  ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE} )
target_include_directories( libswindle PRIVATE ${BMP_EXTRA} ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_include_directories( libswindle PRIVATE ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config )
target_include_directories( libswindle PRIVATE ${myB}/private_include)
target_include_directories( libswindle PUBLIC  ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS} )
target_link_libraries( libswindle lnArduino tinyUsb)

IF(USE_RP2040)
  target_link_libraries( libswindle rplib )
ENDIF()

#---------
#
# rvTap does not like -Oz
# Something to fix here
MESSAGE(STATUS "Restricting flags for rvTap to -Os")
set_property(SOURCE src/bmp_rvTap.cpp  PROPERTY COMPILE_OPTIONS "-Ofast")
#
# ===========================================================================================
