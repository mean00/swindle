#___________________________________________________________
# This builds lnBMP in hosted mode
# same as normal BMP in hosted mode + freeRTOS support
#___________________________________________________________

cmake_minimum_required(VERSION 3.13)
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
PROJECT(lnBMPHosted C CXX)
SET(LN_BMP_HOSTED True CACHE INTERNAL "")

SET(BMP ${CMAKE_SOURCE_DIR}/../blackmagic)
SET(ARDUINO_GD32_FREERTOS ${CMAKE_SOURCE_DIR}/../lnArduino)

include( ${ARDUINO_GD32_FREERTOS}/embeddedPrintf.cmake)

include_directories(include)
include_directories(${BMP}/)
include_directories(${BMP}/src/include)
include_directories(${BMP}/src/platforms/pc)
include_directories(${BMP}/src/platforms/hosted)
include_directories(${BMP}/src/target)
include_directories(${BMP}/../lnBlackmagic/include )

#
#

ADD_DEFINITIONS("-DPC_HOSTED=1")
ADD_DEFINITIONS("-DENABLE_DEBUG=1")
ADD_DEFINITIONS("-g3")
ADD_SUBDIRECTORY(../lnBlackmagic lnBMP)

SET(H ${CMAKE_CURRENT_SOURCE_DIR}/src)
SET(SRC ${H}/main.cpp ${H}/lnDebug.cpp src/stubs.cpp )

ADD_EXECUTABLE(lnBMP_hosted ${SRC})
target_link_libraries(lnBMP_hosted lnBlackmagic embeddedPrintf usb-1.0 )
