
#----------------------------
# Supported boards
#----------------------------

SET(HOSTED ${BMP}/src/platforms/hosted CACHE INTERNAL "")
SET(PC ${BMP}/src/platforms/hosted CACHE INTERNAL "")
SET(LN_EXT "arm_gd32fx" CACHE INTERNAL "")
SET(LN_MCU "M3" CACHE INTERNAL "")

include(./swindle_common.cmake)


# ===========================================================================================
# Hosted mode
# ===========================================================================================
find_package(PkgConfig)


MESSAGE(STATUS "Building for hosted mode (${SWINDLE_HOSTED})")
ADD_DEFINITIONS("-DBMD_IS_STDC=1")
ADD_DEFINITIONS("-DPC_HOSTED=1")
# if the next line is present, wchlink will not work
ADD_DEFINITIONS("-DENABLE_DEBUG=1")

# Set a dummy configuration for lnArduino so it builds
ADD_DEFINITIONS("-DLN_ARCH=LN_ARCH_ARM")
include(${ARDUINO_GD32_FREERTOS}/setup.cmake)
# ===========================================================================================

include_directories(${S})
include_directories(../)
include_directories(../lnArduino/arm_gd32fx/boards/bluepill)
include_directories( ${HOSTED} ${BMP_EXTRA}/hosted/)
include_directories( ${BMP}/src/include)
include_directories( ${BMP}/src)
include_directories( ${BMP}/src/target)
# ===========================================================================================

SET(BM_HOSTED
          ${HOSTED}/bmp_remote.c
          ${HOSTED}/platform.c
          ${HOSTED}/debug.c
          ${HOSTED}/remote/protocol_v0.c
          ${HOSTED}/remote/protocol_v0_swd.c
          ${HOSTED}/remote/protocol_v0_jtag.c
          ${HOSTED}/remote/protocol_v0_adiv5.c
          ${HOSTED}/remote/protocol_v1.c
          ${HOSTED}/remote/protocol_v1_adiv5.c
          ${HOSTED}/remote/protocol_v2.c
          ${HOSTED}/remote/protocol_v3.c
          ${HOSTED}/remote/protocol_v3_adiv5.c
          ${HOSTED}/remote/protocol_v4.c
          ${HOSTED}/remote/protocol_v4_adiv5.c
          ${HOSTED}/remote/protocol_v4_riscv.c

          ${BMP_EXTRA}/hosted/remote_rv_protocol.c
          ${BMP_EXTRA}/hosted/remote_adiv5.c

          ${PC}/utils.c
          ${PC}/cli.c
          ${PC}/probe_info.c
          ${T}/jtag_scan.c
          ${T}/jtag_devs.c
          ${T}/adiv5_jtag.c
          ${T}/spi.c

          CACHE INTERNAL ""
          )
# ===========================================================================================

add_library(libswindle STATIC ${BM_SRC} ${BRIDGE_SRCS}  ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE} )

target_include_directories( libswindle PRIVATE ${BMP_EXTRA} ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_include_directories( libswindle PUBLIC ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS} )
target_include_directories(libswindle PRIVATE  ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config )
target_include_directories(libswindle PRIVATE  ${myB}/private_include)
target_link_libraries( libswindle lnArduino)

# ===========================================================================================
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle/c_interface bmp_c_interface)
include(rnCmake)
corrosion_import_crate(MANIFEST_PATH rs/rs_swindle/Cargo.toml    NO_DEFAULT_FEATURES      FLAGS ${LN_LTO_RUST_FLAGS} )
corrosion_add_target_rustflags( rsbmp --cfg feature="hosted")
# ===========================================================================================
