#___________________________________________________________
# This builds lnBMP in hosted mode
# same as normal BMP in hosted mode + freeRTOS support
#___________________________________________________________

cmake_minimum_required(VERSION 3.13)
SET(SRC bmp_interface_c.cpp bmp_stubs_c.cpp bmp_rpc.cpp)
ADD_LIBRARY(lnBMP_interface STATIC ${SRC})
target_link_libraries( lnBMP_interface lnArduino)
target_include_directories( lnBMP_interface PRIVATE ../../../include)
