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
  # Order matches the historical directory-scope order (BMP core dirs, then
  # swindle/include) so include-search precedence is preserved for every target
  # that links bmp_core. ${CMAKE_BINARY_DIR}/generated holds the configure-time
  # generated platformgenerated.h; it is reachable ONLY through this target, so
  # the header stays scoped to blackmagic-touching targets (it is NOT exposed
  # through any project-wide include_directories). NOTE: ${BMP_EXTRA}
  # (blackmagic_addon) is intentionally NOT here - historically only libswindle
  # saw it (as a PRIVATE include), so it stays a libswindle private include
  # (see swindle_target.cmake).
  target_include_directories(bmp_core INTERFACE
      ${BMP}/src/include
      ${BMP}/src
      ${BMP}/src/target
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
  else()
    set(_pgh_pc_hosted 0)
    set(_pgh_embedded TRUE)
    set(_pgh_enable_debug 0)
    set(_pgh_platform_has_debug 0)
  endif()
  set(_pgh_content "/* platformgenerated.h : blackmagic compile context, generated at configure time. */\n")
  string(APPEND _pgh_content "#ifndef PLATFORMGENERATED_H\n#define PLATFORMGENERATED_H\n")
  string(APPEND _pgh_content "#define ENABLE_RISCV 1\n")
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
# ---------------------------------------------------------------------------
if(NOT TARGET ln_bsp)
  add_library(ln_bsp INTERFACE)
endif()

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
# Supported boards
# ----------------------------
set(BOARDS
    ${enabled_sources} ${BMP_EXTRA}/target/CH32V3xx/ch32v3xx.c ${B}/bmp_disabledBoard.cpp ${T}/lpc_common.c
    CACHE INTERNAL "")

# -------
set(BM_SRC
    ${S}/command.c ${S}/exception.c ${S}/hex_utils.c
    ${S}/maths_utils.c
   # ${S}/timing.c
    # ${S}/rtt.c ${S}/remote.c ${S}/gdb_main.c ${S}/gdb_packet.c ${S}/crc32.c ${S}/morse.c
    CACHE INTERNAL "")
# ------
set(BM_TARGET
    ${T}/adi.c
    ${T}/adiv5.c
    ${T}/adiv5_swd.c
    ${T}/adiv6.c
    ${T}/cortex.c
    ${T}/cortexm.c
    ${T}/target_flash.c
    ${T}/gdb_reg.c
    ${T}/target.c
    ${T}/stm32_common.c
    # ${T}/riscv64.c ${S}/gdb_hostio.c ${T}/jtag_scan.c) ${T}/jtag_devs.c    ${T}/adiv5_jtagdp.c ${T}/cortexa.c
    # ${T}/lmi.c ${T}/target_probe.c
    CACHE INTERNAL "")

if(SMALL_SWINDLE)
  set(BM_TARGET ${BM_TARGET}
      src/small_swindle_stub.cpp
  )
else()
  set(BM_TARGET ${BM_TARGET}
    ${BMP_EXTRA}/extra/riscv_memaccess.c
    ${T}/riscv_adi_dtm.c
    ${T}/riscv_debug.c
    ${T}/riscv32.c
    ${T}/sfdp.c
    ${T}/spi.c
  )
endif()


# ---------
