# ___________________________________________________________ This builds lnBMP in hosted mode same as normal BMP in
# hosted mode + freeRTOS support ___________________________________________________________

cmake_minimum_required(VERSION 3.13)
# The SDI support is optional (SWINDLE_WITH_SDI, top level CMakeLists.txt): the
# c_interface side of it is the 'mon sdi_wire' knob setter, and this is the one
# place the option changes (see swindle/include/bmp_riscv_extra.h for the seam).
if(SWINDLE_WITH_SDI)
  set(SDI_SRC bmp_riscv_extra_sdi_c.cpp)
else()
  # The no-op twin (bmp_riscv_extra_sdi_stubs_c.cpp) is Rust now: see
  # swindle/rs/rs_swindle/src/riscv_extra_sdi_stubs.rs, bmp_set_sdi_wire_c.
  set(SDI_SRC)
endif()
# bmp_stubs_c.cpp is Rust now (rs_swindle/src/bmp_stubs.rs, same two symbols, same
# `xAssert(0)` behaviour): the C file only kept `vasprintf_()` and
# `gdb_putpacket_f()` defined, nothing called them.
set(SRC bmp_interface_c.cpp ${SDI_SRC})
# bmp_rtt.cpp
add_library(swindle_interface STATIC ${SRC})
target_link_libraries(swindle_interface PRIVATE esprit_dev)
target_link_libraries(swindle_interface PRIVATE bmp_core)
ln_force_platformgen(swindle_interface)
target_include_directories(swindle_interface PRIVATE ../../../include)

if(NOT SWINDLE_USE_NETWORK AND NOT LN_RUST_USB)
  # bmp_interface_usb.cpp held `swindleRedirectLog_c` alone; with LN_RUST_USB it is the
  # Rust `usb/log_redirect.rs` export instead (usb.md P4), and with SWINDLE_USE_NETWORK
  # the no-op export lives in `net/mod.rs`, so neither C file is needed.
  target_sources(swindle_interface PRIVATE bmp_interface_usb.cpp)
endif()
