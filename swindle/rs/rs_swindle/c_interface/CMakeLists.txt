#___________________________________________________________
# This builds lnBMP in hosted mode
# same as normal BMP in hosted mode + freeRTOS support
#___________________________________________________________

cmake_minimum_required(VERSION 3.13)
IF(SWINDLE_HOSTED)
  SET(RPC bmp_rpc_hosted.cpp)
ELSE()
  SET(RPC bmp_rpc.cpp)
ENDIF()
SET(SRC bmp_interface_c.cpp bmp_stubs_c.cpp ${RPC})
#bmp_rtt.cpp
ADD_LIBRARY(swindle_interface STATIC ${SRC})
target_link_libraries( swindle_interface PRIVATE esprit_dev)
target_include_directories( swindle_interface PRIVATE ../../../include)

if(SWINDLE_USE_NETWORK)
  target_sources(swindle_interface PRIVATE bmp_interface_net.cpp)
else()
  target_sources(swindle_interface PRIVATE bmp_interface_usb.cpp)
endif()
