# The WCH SDI support is optional and self-contained (SWINDLE_WITH_SDI, top level
# CMakeLists.txt): every SDI source is named *_riscv_extra.* and the list below is
# the whole of it, plus ../../template/sdi_template.h and the 'mon sdi_wire' knobs
# in swindle/rs/rs_swindle/c_interface. With the option OFF, the two hooks the pin
# mode switch calls (sdi_pinmode_enter/leave) and the SDI entry points (sdi_scan,
# bmp_sdi_dm_*) come from the shared no-op stub file instead, so the SWD/RVSWD
# probe is unchanged and 'mon sdi_scan' simply reports failure.
#
# Both LN SDI taps implement the same session (sdi_scan(), sdi_pinmode_enter() /
# sdi_pinmode_leave() and the sdi_dm_* / ch32_sdi_dmi_* entry points the shared
# protocol layer defines), on the same wire (PB8/PC3) and over the same protocol
# layer (swindle/src/template/sdi_template.h), so exactly one of them is built:
#
#   bmp_sdiTap_ln_timer_riscv_extra.cpp
#                              timer polled: lnTimerWatch times the cells and the
#                              CPU bit-bangs both frames against it, the pad being
#                              push-pull while the probe drives and open drain for
#                              the response cells. This is the one that is built
#                              (SDI_SOURCES below).
#   bmp_sdiTap_ln_bitbang_riscv_extra.cpp
#                              CPU only: every edge of both frames is made by the
#                              CPU, with a counted NOP loop as the clock. No timer
#                              channel, no DMA, and nothing that can block (the
#                              DMA path can park in lnDMA::beginTransfer()'s
#                              channel mutex while running with interrupts masked
#                              - see the file header).
#   bmp_sdiTap_ln_riscv_extra.cpp
#                              the timer driven carrier: one compare event per
#                              cell, the per-cell duty written by the CPU a
#                              boundary ahead, so the cell length is the timer's
#                              whatever the CPU is doing. No DMA transfer at all
#                              (the level shifter's turn-around is two CPU stores
#                              inside each response cell). Kept in the tree as the
#                              hardware paced reference the bit-bang tap's
#                              comments are written against; swap the two names
#                              below to build it again.
#
# Both taps' per-cell paths are latency critical, for the same reason and with
# different budgets: the bit-bang tap's clock is a counted NOP loop, so the cell's
# cost *is* the cell, while the timer tap makes the edges itself and only has to
# keep its per-cell *tail* inside the cell - the pad read, the shifter turn-back
# and the next boundary poll share the `period - sample` ticks after the sample
# (60 of 86 here), which is what the turn-back store has to land inside or the
# next read clock's LOW never reaches the target. The target platform flags
# optimize for size (-Oz), where clang keeps the small helpers out of line and
# every extra bus access is a measurable part of that budget. These are the two
# files that need the speed, so it is a source property here rather than a build
# type for the whole firmware, whose SWD timing was brought up with the size flags.
if(SWINDLE_WITH_SDI)
  set_source_files_properties(bmp_sdiTap_ln_timer_riscv_extra.cpp PROPERTIES COMPILE_OPTIONS "-O2")
  set_source_files_properties(bmp_sdiTap_ln_bitbang_riscv_extra.cpp PROPERTIES COMPILE_OPTIONS "-O2")
  set_source_files_properties(bmp_sdiTap_ln_riscv_extra.cpp PROPERTIES COMPILE_OPTIONS "-O2")
  set(SDI_SOURCES bmp_sdiTap_ln_timer_riscv_extra.cpp)
else()
  set(SDI_SOURCES ../../template/bmp_riscv_extra_stubs.cpp)
endif()

add_library(swindleio_impl STATIC bmp_adc_ln.cpp bmp_rvTap_ln.cpp bmp_swdTap_ln.cpp bmp_tap_ln.cpp
                                  ${SDI_SOURCES}
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
# bmp_sdiTap_ln_riscv_extra.cpp. ESPRIT_ROOT is set by the top level
# CMakeLists.txt before add_subdirectory(swindle).
target_include_directories(swindleio_impl PRIVATE ${ESPRIT_ROOT}/private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)
ln_setup_platform_impl(swindleio_impl)
