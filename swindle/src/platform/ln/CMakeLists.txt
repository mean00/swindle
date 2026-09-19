# Both LN SDI taps implement the same session (sdi_scan(), sdi_pinmode_enter() /
# sdi_pinmode_leave() and the sdi_dm_* / ch32_sdi_dmi_* entry points the shared
# protocol layer defines), on the same wire (PB8/PC3) and over the same protocol
# layer (swindle/src/template/sdi_template.h), so exactly one of them is built:
#
#   bmp_sdiTap_ln_bitbang.cpp  CPU only: every edge of both frames is made by the
#                              CPU, with a counted NOP loop as the clock. No timer
#                              channel, no DMA, and nothing that can block (the
#                              DMA path can park in lnDMA::beginTransfer()'s
#                              channel mutex while running with interrupts masked
#                              - see the file header). This is the one that is
#                              built.
#   bmp_sdiTap_ln.cpp          the timer + DMA carrier: PWM write frames, a CPU
#                              clocked read frame and DMA driven DIR
#                              turn-arounds. Kept in the tree as the waveform the
#                              bit-bang tap aims at, and as the timing reference
#                              its comments are written against; swap the two
#                              names below to build it again.
#
# The bit-bang tap's clock is that loop, so the cell's cost *is* the cell: the
# per-cell helpers have to stay in registers and the loop's state out of memory.
# The target platform flags optimize for size (-Oz), where clang keeps the small
# helpers out of line and every extra bus access is a measurable part of the few
# dozen iterations a cell is made of. This is the file that needs the speed, so it
# is a source property here rather than a build type for the whole firmware, whose
# SWD timing was brought up with the size flags.
set_source_files_properties(bmp_sdiTap_ln_bitbang.cpp PROPERTIES COMPILE_OPTIONS "-O2")

add_library(swindleio_impl STATIC bmp_adc_ln.cpp bmp_rvTap_ln.cpp bmp_swdTap_ln.cpp bmp_tap_ln.cpp
                                  bmp_sdiTap_ln_bitbang.cpp
                                  ../../template/swd_tap_stubs.cpp)
# Board package selection -> ln_bsp interface (linked PUBLIC into libswindle by
# swindle_target.cmake and PRIVATE into swindleio_impl here).
if(USE_48PIN_PACKAGE)
  target_compile_definitions(ln_bsp INTERFACE USE_48PIN_PACKAGE)
elseif(USE_64PIN_PACKAGE)
  target_compile_definitions(ln_bsp INTERFACE USE_64PIN_PACKAGE)
else()
  message(FATAL_ERROR "You must select either USE_48PIN_PACKAGE or USE_64PIN_PACKAGE")
endif()
target_include_directories(swindleio_impl PRIVATE ../..)
target_include_directories(swindleio_impl PRIVATE ../../template)
target_include_directories(swindleio_impl PRIVATE ../../../private_include)
# esprit private headers (lnTimer_priv.h - timer registers and bit masks,
# lnPinMapping.h - the timer/channel -> DMA engine/channel table), needed by
# bmp_sdiTap_ln.cpp. ESPRIT_ROOT is set by the top level CMakeLists.txt before
# add_subdirectory(swindle).
target_include_directories(swindleio_impl PRIVATE ${ESPRIT_ROOT}/private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)
ln_setup_platform_impl(swindleio_impl)
