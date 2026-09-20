# ___________________________________________________________ This builds lnBMP in hosted mode same as normal BMP in
# hosted mode + freeRTOS support ___________________________________________________________

cmake_minimum_required(VERSION 3.13)
if(SWINDLE_HOSTED)
  set(RPC bmp_rpc_hosted.cpp)
else()
  set(RPC bmp_rpc.cpp)
endif()
# The SDI support is optional (SWINDLE_WITH_SDI, top level CMakeLists.txt): the
# c_interface side of it is the 'mon sdi_wire' knob setter, and this is the one
# place the option changes (see swindle/include/bmp_riscv_extra.h for the seam).
if(SWINDLE_WITH_SDI)
  set(SDI_SRC bmp_riscv_extra_sdi_c.cpp)
else()
  set(SDI_SRC bmp_riscv_extra_sdi_stubs_c.cpp)
endif()
set(SRC bmp_interface_c.cpp ${SDI_SRC} bmp_stubs_c.cpp ${RPC})
# bmp_rtt.cpp
add_library(swindle_interface STATIC ${SRC})
target_link_libraries(swindle_interface PRIVATE esprit_dev)
target_link_libraries(swindle_interface PRIVATE bmp_core)
ln_force_platformgen(swindle_interface)
target_include_directories(swindle_interface PRIVATE ../../../include)

if(SWINDLE_USE_NETWORK)
  target_sources(swindle_interface PRIVATE bmp_interface_net.cpp)
else()
  target_sources(swindle_interface PRIVATE bmp_interface_usb.cpp)
endif()
