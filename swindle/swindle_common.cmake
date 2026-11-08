set(S
    ${BMP}/src
    CACHE INTERNAL "")
set(T
    ${BMP}/src/target
    CACHE INTERNAL "")
set(P
    ${BMP}/src/platforms
    CACHE INTERNAL "")
set(myB
    ${CMAKE_CURRENT_SOURCE_DIR}/
    CACHE INTERNAL "")
set(B
    ${myB}/src
    CACHE INTERNAL "")

# ---------------------------------------------------------------------------
# bmp_core : the shared blackmagic / blackmagic_addon C compile context.
#
# Historically this context (-DENABLE_RISCV -DCONFIG_* -DNO_LIBOPENCM3=1 ... and
# the ${BMP}/src{,/include,/target} include folders) was pushed into every
# target of the swindle directory through directory-scope add_definitions() and
# include_directories(). It is now carried by ONE named INTERFACE target that
# each target compiling blackmagic sources links explicitly (PRIVATE):
#   - libswindle        (swindle_target.cmake / swindle_hosted.cmake)
#   - swindleio_impl    (swindle/src/platform/*/CMakeLists.txt)
#   - swindle_interface (swindle/rs/rs_swindle/c_interface/CMakeLists.txt)
#
# The blackmagic context DEFINES no longer travel as -D flags: they are baked
# into the configure-time generated header platformgenerated.h (see
# ln_generate_platform_header() below), force-included into each consumer by
# ln_force_platformgen(). bmp_core therefore only carries the include folders.
# ---------------------------------------------------------------------------
set(BMP_EXTRA ${B}/../../blackmagic_addon/)

if(NOT TARGET bmp_core)
  add_library(bmp_core INTERFACE)
  # Order matches the historical directory-scope order (the private shim headers
  # of swindle/private_include, which stand in for blackmagic's src{,/include,
  # /target} folders, then swindle/include) so include-search precedence is
  # preserved for every target that links bmp_core. ${CMAKE_BINARY_DIR}/generated
  # holds the configure-time generated platformgenerated.h; it is reachable ONLY
  # through this target, so the header stays scoped to blackmagic-touching
  # targets (it is NOT exposed through any project-wide include_directories).
  # NOTE: ${BMP_EXTRA} (blackmagic_addon) is intentionally NOT here - historically
  # only libswindle saw it (as a PRIVATE include), so it stays a libswindle
  # private include (see swindle_target.cmake).
  target_include_directories(bmp_core INTERFACE
      ${myB}/private_include
      ${myB}/include
      ${CMAKE_BINARY_DIR}/generated)
endif()

# ---------------------------------------------------------------------------
# ln_generate_platform_header() : write platformgenerated.h at configure time.
#
# platformgenerated.h is the single force-include entry point for every target
# compiling blackmagic sources AND the single root of truth for the whole
# blackmagic compile context. It merges what used to be (a) the bmp_core
# INTERFACE_COMPILE_DEFINITIONS block (ENABLE_RISCV, CONFIG_*, NO_LIBOPENCM3,
# PC_HOSTED, BMD_IS_STDC, PLATFORM_IDENT), (b) the miniplatform.h
# force-include, and (c) the ENABLE_DEBUG / ENABLE_RISCV / PLATFORM_HAS_DEBUG
# values that used to be (re)defined by miniplatform.h's SMALL_SWINDLE block
# and by swindle/include/platform.h. Those two headers no longer define ANY of
# these macros: the generated header is the only place they are defined.
#
# The emitted values reproduce exactly what was effectively in force before
# the merge, so the preprocessed output (and the firmware) is unchanged:
#   - embedded: ENABLE_DEBUG 0 and PLATFORM_HAS_DEBUG 0 (miniplatform.h
#     hardcoded SMALL_SWINDLE=1 for every embedded build, overriding the
#     -DENABLE_DEBUG=1 default), ENABLE_RISCV 1 (platform.h redefined it back
#     to 1 after miniplatform.h's 0).
#   - hosted:   ENABLE_DEBUG 1, PLATFORM_HAS_DEBUG 1, ENABLE_RISCV 1.
# The hosted variant carries the hosted values and deliberately does NOT
# include miniplatform.h (hosted never had it: it would pull
# Logger/snprintf_/printf.h into the hosted libswindle).
# ---------------------------------------------------------------------------
function(ln_generate_platform_header)
  if(SWINDLE_HOSTED)
    set(_pgh_pc_hosted 1)
    set(_pgh_embedded FALSE)
    set(_pgh_enable_debug 1)
    set(_pgh_platform_has_debug 1)
    set(_pgh_pointer_size 8)
  else()
    set(_pgh_pc_hosted 0)
    set(_pgh_embedded TRUE)
    set(_pgh_enable_debug 0)
    set(_pgh_platform_has_debug 0)
    set(_pgh_pointer_size 4)
  endif()
  set(_pgh_content "/* platformgenerated.h : blackmagic compile context, generated at configure time. */\n")
  string(APPEND _pgh_content "#ifndef PLATFORMGENERATED_H\n#define PLATFORMGENERATED_H\n")
  string(APPEND _pgh_content "#define ENABLE_RISCV 1\n")
  string(APPEND _pgh_content "#define CONFIG_RP\n")
  string(APPEND _pgh_content "#define CONFIG_NXP\n")
  string(APPEND _pgh_content "#define CONFIG_NRF\n")
  string(APPEND _pgh_content "#define CONFIG_RISCV\n")
  string(APPEND _pgh_content "#define CONFIG_GD32\n")
  string(APPEND _pgh_content "#define CONFIG_MM32\n")
  string(APPEND _pgh_content "#define CONFIG_RVSWD\n")
  string(APPEND _pgh_content "#define PLATFORM_HAS_RVSWD\n")
  string(APPEND _pgh_content "#define NO_LIBOPENCM3 1\n")
  string(APPEND _pgh_content "#define ENABLE_DEBUG ${_pgh_enable_debug}\n")
  string(APPEND _pgh_content "#define PLATFORM_HAS_DEBUG ${_pgh_platform_has_debug}\n")
  string(APPEND _pgh_content "#define PC_HOSTED ${_pgh_pc_hosted}\n")
  string(APPEND _pgh_content "#define BMD_IS_STDC 1\n")
  string(APPEND _pgh_content "#define CONFIG_POINTER_SIZE ${_pgh_pointer_size}\n")
  string(APPEND _pgh_content "#ifdef __cplusplus\n#ifndef _Static_assert\n#define _Static_assert static_assert\n#endif\n#endif\n")
  if(_pgh_embedded)
    string(APPEND _pgh_content "#define PLATFORM_IDENT \"lnBMP\"\n")
    string(APPEND _pgh_content "#include \"miniplatform.h\"\n")
  endif()
  string(APPEND _pgh_content "#endif /* PLATFORMGENERATED_H */\n")
  file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/generated)
  file(WRITE ${CMAKE_BINARY_DIR}/generated/platformgenerated.h "${_pgh_content}")
endfunction()
ln_generate_platform_header()

# ---------------------------------------------------------------------------
# ln_bsp : board-selection interface.
# The board defines (USE_RP_CARRIER, LN_UART_*, USE_48PIN_PACKAGE,
# USE_64PIN_PACKAGE ...) used to be sprayed as PUBLIC target_compile_definitions
# on swindleio_impl. They now travel through this one interface, linked
# PUBLIC into libswindle and PRIVATE into swindleio_impl.
# The pinout / ADC / UART / USB / LED values themselves are no longer defines: they
# come from the probe board table as bmp_pinout_board.h (board.cmake below). What
# is left here is the selection flags of the platforms still on their C pinout
# (RP2040/RP2350: USE_RP_CARRIER + LN_UART_*, ESP32: LN_ESP_MINI).
# ---------------------------------------------------------------------------
if(NOT TARGET ln_bsp)
  add_library(ln_bsp INTERFACE)
endif()

# ---------------------------------------------------------------------------
# Probe board table -> generated bmp_pinout_board.h (see board.cmake).
# Picks the board file (swindle/rs/rs_swindle_board/src/<board>.rs) and renders it
# for C, so the pinout / ADC / UART / USB / LED numbers live in Rust only. Sets
# SWINDLE_BOARD (also the cargo feature) and the swindle_board_header target.
# ---------------------------------------------------------------------------
include(./board.cmake)

# ---------------------------------------------------------------------------
# GCC build path: force-include platformgenerated.h once, at directory scope.
#
# platformgenerated.h is forced in on the clang path through
# ln_force_platformgen() (per-target joined spelling below). That spelling is
# clang-only: GCC's `-include` is Separate-only and the GCC toolchains we build
# with (arm-none-eabi-gcc 15.2.1, riscv-none-elf-gcc 14.2.0) reject
# `-includeplatformgenerated.h`. The separate form cannot be used as a per-target
# compile OPTION either: when another `-include` option is present (RP2040 /
# RP2350: esprit_dev force-includes fix_sev.h through its INTERFACE options)
# the Makefile generator deduplicates the `-include` flag and merges the file
# arguments into one `-include a b`, which makes the second file an input
# source for BOTH compilers. The historical add_definitions() channel (raw
# COMPILE_DEFINITIONS, never merged against interface options) coexists
# cleanly with esprit_dev, so GCC builds reproduce it verbatim - once, at this
# directory scope and before any subdirectory is added (exactly what the
# pre-refactor build did). Verified empirically in a minimal CMake repro:
#   target/interface options: -include a -include b  -> -include a b   (broken)
#   add_definitions channel:  -include a -include b  -> -include a -include b (ok)
# The header resolves through the include path (${CMAKE_BINARY_DIR}/generated,
# provided by bmp_core, which every blackmagic-touching target links).
# ---------------------------------------------------------------------------
if(NOT SWINDLE_HOSTED AND CMAKE_C_COMPILER_ID STREQUAL "GNU")
  add_definitions("-include platformgenerated.h")
endif()

# ---------------------------------------------------------------------------
# ln_force_platformgen(tgt) : force-include the generated platformgenerated.h
# into one blackmagic-touching target.
#
# Kept OUT of bmp_core's INTERFACE_COMPILE_OPTIONS on purpose: the Makefile
# generator deduplicates the `-include` flag whenever more than one option uses
# it (bmp_core's platformgenerated.h + esprit_dev's fix_sev.h on RP2040) and
# merges the file arguments into one `-include a b`, which makes clang treat
# "b" as an input source. The header must resolve through the include path
# (${CMAKE_BINARY_DIR}/generated, provided by bmp_core).
#
# Three channels, matching the three build paths:
#   - Hosted: hosted bmp_core carries no -D defines anymore and hosted has no
#     directory-scope channel, so the hosted-variant header must be forced in
#     per-target. The separate `-include;platformgenerated.h` spelling is safe
#     here: hosted has no esprit_dev `-include`, so no merge can happen, and
#     host GCC / clang both accept it.
#   - GNU embedded: covered once by the swindle/ directory-scope
#     add_definitions() (see above); a per-target option would collide with
#     esprit_dev's `-include fix_sev.h` on RP2040/RP2350 and merge.
#   - clang embedded: joined spelling `-includeplatformgenerated.h` (NOT the
#     two-arg `-include platformgenerated.h`): the Makefile generator
#     deduplicates the `-include` flag when several options use it (esprit_dev
#     force-includes fix_sev.h the same way on RP2040) and merges the file
#     arguments into one `-include a b`, which makes clang treat "b" as an
#     input source. The joined form is a distinct token, so no merge happens.
#     clang accepts `-include<file>` (JoinedOrSeparate).
# ---------------------------------------------------------------------------
function(ln_force_platformgen tgt)
  if(SWINDLE_HOSTED)
    target_compile_options(${tgt} PRIVATE "-include;platformgenerated.h")
    return()
  endif()
  if(CMAKE_C_COMPILER_ID STREQUAL "GNU")
    return() # covered by the directory-scope add_definitions() above
  endif()
  target_compile_options(${tgt} PRIVATE "-includeplatformgenerated.h")
endfunction()

# ---------------------------------------------------------------------------
# ln_setup_platform_impl(tgt) : wire the shared context every swindleio_impl
# target needs.
#
# Every platform implementation (src/platform/ln, rp2040, esp32_*) is a
# `swindleio_impl` target that must link the blackmagic context (bmp_core: BMP
# include dirs + generated platformgenerated.h), the board-package interface
# (ln_bsp: USE_48PIN_*, LN_UART_*, ...) and the platformgenerated.h
# force-include. Centralizing the trio keeps the per-platform CMakeLists free
# of the invariant (a new platform gets all three at once instead of silently
# missing one). Note this does NOT cover esprit_dev / the ../.. includes: those
# are per-platform boilerplate that predates this migration.
# ---------------------------------------------------------------------------
function(ln_setup_platform_impl tgt)
  target_link_libraries(${tgt} PRIVATE bmp_core)
  target_link_libraries(${tgt} PRIVATE ln_bsp)
  ln_force_platformgen(${tgt})
endfunction()

# ----------------------------
# Pure Rust target drivers (rs_stm32f1f4 / rs_stm32g0 / rs_rp2040 / rs_rp2350_arm)
# plus the RISC-V ones (rs_ch32v00x / rs_ch32v123xx / rs_rp2350_riscv)
# ----------------------------
# The probe chain (rs_swindle_wrapper/src/arm_probe_chain.rs) calls the Rust driver of a
# ported family instead of the C probe of the same name, so that C probe is dead code and
# the linker drops it.
#
# SWINDLE_RUST_ARM_TARGETS lists the ported families whose C source is built, so the Rust
# half is only enabled when the target it probes exists; a family that is not listed keeps
# its C probe in the chain.
#
# SWINDLE_RUST_ONLY_FAMILIES are the families whose C file holds nothing but probes that are
# Rust now: with the Rust driver on, the file is left out of the build altogether. All the
# ported ones are in the list — rp2040 included, since its ADIv5 rescue path
# (`rp2040_rescue_probe`, called from `adiv5.c`) is Rust too now
# (`rs_swindle/src/arm/rescue.rs`, exported by the wrapper).
#
# Dropping rp2040.c/rp2350.c also drops the last `bmp_spi_add_flash()` caller of those two
# families, which is what decides whether `spi.c`/`sfdp.c` are still needed (see
# SWINDLE_NEEDS_SPI_FLASH below).
#
# ch32f1.c is dropped by the block below the loop rather than by the loop: it is an STM32F1
# clone whose driver is Rust with the rest of the family, but it has no cargo feature of its
# own (the chain keys it off `rust_stm32f1`), so it cannot be a name in the list above.
set(SWINDLE_RUST_ONLY_FAMILIES stm32f1 stm32f4 stm32g0 rp2040 rp2350 nrf51 nrf54l nrf91)
set(SWINDLE_RUST_ARM_TARGETS "" CACHE INTERNAL "")

foreach(family stm32f1 stm32f4 stm32g0 rp2040 rp2350 nrf51 nrf54l nrf91)
  list(FIND enabled_sources ${T}/${family}.c ${family}_source_idx)
  if(${family}_source_idx EQUAL -1)
    message(STATUS "Rust ${family} driver: OFF (C ${family}.c not built)")
    continue()
  endif()
  list(APPEND SWINDLE_RUST_ARM_TARGETS ${family})
  if(family IN_LIST SWINDLE_RUST_ONLY_FAMILIES)
    list(REMOVE_ITEM enabled_sources ${T}/${family}.c)
    message(STATUS "Rust ${family} driver: ON (C ${family}.c not compiled)")
  else()
    message(STATUS "Rust ${family} driver: ON")
  endif()
endforeach()
set(SWINDLE_RUST_ARM_TARGETS "${SWINDLE_RUST_ARM_TARGETS}" CACHE INTERNAL "")

# The CH32F1 driver is the same Rust code as the rest of the family (rs_stm32f1f4), and its
# board is always enabled with STM32F1, so ch32f1.c goes when the family is Rust.
if("stm32f1" IN_LIST SWINDLE_RUST_ARM_TARGETS)
  list(FIND enabled_sources ${T}/ch32f1.c ch32f1_source_idx)
  if(NOT ch32f1_source_idx EQUAL -1)
    list(REMOVE_ITEM enabled_sources ${T}/ch32f1.c)
    message(STATUS "Rust ch32f1 driver: ON (C ch32f1.c not compiled)")
  endif()
endif()

# ----------------------------
# bmp_spi_add_flash() users (spi.c / sfdp.c)
# ----------------------------
# spi.c is the SPI-flash manager (`bmp_spi_add_flash()`) and sfdp.c the parameter reader it
# calls; a driver that programs its flash through a controller uses them to get a
# `target_flash_s` without writing its own. The Rust RP drivers do not: they drive the QMI
# directly (rs_rp_common/src/{spi,sfdp}.rs). So the pair is only compiled while at least one
# family that still asks for it is built — with every family below ported or disabled, both
# files leave the build with the C drivers that used them.
set(SWINDLE_SPI_FLASH_USERS
    ${T}/sfdp.c
    #${T}/imxrt.c
    #${T}/lpc43xx.c
    #${T}/renesas_rz.c
    ${T}/rp2040.c
    ${T}/rp2350.c)
set(SWINDLE_NEEDS_SPI_FLASH OFF)
foreach(src ${SWINDLE_SPI_FLASH_USERS})
  if(src IN_LIST enabled_sources)
    set(SWINDLE_NEEDS_SPI_FLASH ON)
  endif()
endforeach()
if(SWINDLE_NEEDS_SPI_FLASH)
  message(STATUS "SPI flash manager: ON (an enabled driver uses bmp_spi_add_flash)")
else()
  message(STATUS "SPI flash manager: OFF (no driver uses bmp_spi_add_flash)")
endif()

# ----------------------------
# Supported boards
# ----------------------------
set(BOARDS
    ${enabled_sources}
    ${B}/bmp_disabledBoard.cpp
    #${T}/lpc_common.c
    CACHE INTERNAL "")

# -------
# BM_SRC is now empty: exception.c, hex_utils.c, and maths_utils.c are replaced by bmp_helpers.cpp
set(BM_SRC
    CACHE INTERNAL "")
# ------
# BM_TARGET is now empty: adi.c, adiv5.c, adiv5_swd.c, and adiv6.c are replaced by rs_swindle::arm::adi
set(BM_TARGET
    CACHE INTERNAL "")

if(SMALL_SWINDLE)
  set(BM_TARGET ${BM_TARGET}
      src/small_swindle_stub.cpp
  )
else()
  set(BM_TARGET ${BM_TARGET}
    #${T}/riscv_adi_dtm.c
    # ${T}/riscv_debug.c (replaced by rs_swindle::riscv)
    # ${T}/riscv32.c (replaced by rs_swindle::riscv)
  )
  # Only for the drivers that still add their flash through bmp_spi_add_flash()
  # (see SWINDLE_NEEDS_SPI_FLASH above).
  if(SWINDLE_NEEDS_SPI_FLASH)
    set(BM_TARGET ${BM_TARGET}
      ${T}/sfdp.c
      ${T}/spi.c
    )
  endif()
endif()


# ---------
