#___________________________________________________________
# This builds lnBMP in hosted mode
# same as normal BMP in hosted mode + freeRTOS support
#___________________________________________________________

cmake_minimum_required(VERSION 3.13)
SET(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
PROJECT(lnBMPHosted C CXX)
SET(LN_BMP_HOSTED True CACHE INTERNAL "")
SET(LN_ARCH            "X86" CACHE INTERNAL "")
SET(LN_MCU             "M4" CACHE INTERNAL "")
SET(LN_TOOLCHAIN_EXT   "arm_gd32fx_clang" CACHE INTERNAL "")
SET(LN_ENABLE_RUST      ON   CACHE INTERNAL "")
set(AF_FOLDER          ${CMAKE_SOURCE_DIR}/../lnArduino CACHE INTERNAL "")
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${AF_FOLDER}/cmake CACHE INTERNAL "")

SET(Rust_CARGO_TARGET "" CACHE INTERNAL "")
SET(CMAKE_C_COMPILER /usr/bin/clang CACHE INTERNAL "")
SET(CMAKE_CXX_COMPILER /usr/bin/clang++ CACHE INTERNAL "")
SET(CMAKE_CXX_AR /usr/bin/llvm-ar  CACHE INTERNAL "")
SET(CMAKE_CXX_RANLIB /usr/bin/llvm-ranlib  CACHE INTERNAL "")

# Qt5

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)


find_package(Qt5 COMPONENTS Network SerialPort REQUIRED)


SET(BMP ${CMAKE_SOURCE_DIR}/../blackmagic)
SET(ARDUINO_GD32_FREERTOS ${CMAKE_SOURCE_DIR}/../lnArduino)
ADD_DEFINITIONS("-DLN_LINUX=1")
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

#
include(rnCmake)
corrosion_import_crate(MANIFEST_PATH ../rs/lnbmp/Cargo.toml    NO_DEFAULT_FEATURES      CFLAGS ${LN_LTO_RUST_FLAGS} )
corrosion_add_target_rustflags( rsbmp --cfg feature="hosted")

SET(H ${CMAKE_CURRENT_SOURCE_DIR}/src)
SET(SRC ${H}/main.cpp ${H}/lnDebug.cpp ${H}/stubs.cpp ${H}/qtcp.h ${H}/bmp_qtserial.cpp)

ADD_EXECUTABLE(lnBMP_hosted ${SRC})
target_link_libraries(lnBMP_hosted rsbmp)
target_link_libraries(lnBMP_hosted lnBMP_interface)
target_link_libraries(lnBMP_hosted lnBlackmagic  )
target_link_libraries(lnBMP_hosted lnBMP_interface)
target_link_libraries(lnBMP_hosted lnBlackmagic  )
target_link_libraries(lnBMP_hosted embeddedPrintf usb-1.0 )
target_link_libraries(lnBMP_hosted Qt5::Network Qt5::SerialPort)
