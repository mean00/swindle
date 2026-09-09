# ----------------------------
# Supported boards
# ----------------------------

set(HOSTED
    ${BMP}/src/platforms/hosted
    CACHE INTERNAL "")
set(PC
    ${BMP}/src/platforms/hosted
    CACHE INTERNAL "")
set(LN_EXT
    "arm_gd32fx"
    CACHE INTERNAL "")
set(LN_MCU
    "M3"
    CACHE INTERNAL "")

include(./swindle_common.cmake)

# ===========================================================================================
# Hosted mode
# ===========================================================================================
find_package(PkgConfig)

message(STATUS "Building for hosted mode (${SWINDLE_HOSTED})")
# The blackmagic context (BMD_IS_STDC, PC_HOSTED, ENABLE_DEBUG, ENABLE_RISCV,
# ...) is no longer passed as -D flags: the hosted-variant platformgenerated.h
# (force-included into libswindle / swindle_interface via ln_force_platformgen)
# is the single root of truth for it.

# Set a dummy configuration for epsrit so it builds
add_definitions("-DLN_ARCH=LN_ARCH_ARM")
include(${ARDUINO_GD32_FREERTOS}/setup.cmake)
# ===========================================================================================

include_directories(${S})
include_directories(../)
include_directories(../esprit/arm_gd32fx/boards/bluepill)
include_directories(${HOSTED} ${BMP_EXTRA}/hosted/)
# ${BMP}/src{,/include,/target} now come from bmp_core (linked below).
# ===========================================================================================

set(BM_HOSTED
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
    ${BMP_EXTRA}/hosted/remote_sdi_protocol.c
    ${BMP_EXTRA}/hosted/remote_adiv5.c
    ${PC}/utils.c
    ${PC}/cli.c
    ${PC}/probe_info.c
    ${T}/jtag_scan.c
    ${T}/jtag_devs.c
    ${T}/adiv5_jtag.c
    ${T}/spi.c
    CACHE INTERNAL "")
# ===========================================================================================

add_library(libswindle STATIC)
target_sources(libswindle PRIVATE ${BM_SRC} ${BRIDGE_SRCS} ${BOARDS} ${BM_TARGET} ${BM_HOSTED} ${EXTRA_SOURCE})
# blackmagic_addon (${BMP_EXTRA}) stays a libswindle-only include; the rest of
# the blackmagic context comes from bmp_core (linked below).
target_include_directories(libswindle PRIVATE ${BMP_EXTRA})
target_include_directories(libswindle PUBLIC ${usb_INCLUDE_DIRS} ${ftdi_INCLUDE_DIRS})
target_include_directories(libswindle PRIVATE ${B}/include ${CMAKE_BINARY_DIR}/config)
target_include_directories(libswindle PRIVATE ${myB}/private_include)
target_link_libraries(libswindle PRIVATE esprit_dev)
target_link_libraries(libswindle PRIVATE esprit)
target_link_libraries(libswindle PRIVATE bmp_core)
# Hosted bmp_core carries no -D defines anymore: the hosted-variant
# platformgenerated.h (PC_HOSTED=1, no PLATFORM_IDENT, no miniplatform.h) is
# force-included here. swindle_interface gets the same via its own CMakeLists.
ln_force_platformgen(libswindle)

# ===========================================================================================
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle/c_interface bmp_c_interface)
include(rnCmake)
message(STATUS "Rust, enabling hosted feature")
# RUST_ADD( rsbmp rs/rs_swindle/Cargo.toml    "${LN_LTO_RUST_FLAGS}" "hosted")
corrosion_import_crate(MANIFEST_PATH rs/rs_swindle_wrapper/Cargo.toml FLAGS ${LN_LTO_RUST_FLAGS})
corrosion_set_features(rsbmp_wrapper NO_DEFAULT_FEATURES FEATURES hosted)
corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="hosted")
# ===========================================================================================
