#___________________________________________________________
# This builds lnBMP in hosted mode
# same as normal BMP in hosted mode + freeRTOS support
#___________________________________________________________

cmake_minimum_required(VERSION 3.13)
SET(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
PROJECT(lnBMPHosted C CXX)

#
# Create a dummy setup to build BMP/lnBMP
#
set(LN_EXT                  arm_gd32fx)
set(LN_MCU_FOLDER           ${CMAKE_SOURCE_DIR}/../lnArduino/mcus/arm_gd32fx/)
SET(SWINDLE_HOSTED           True CACHE INTERNAL "")
SET(LN_ARCH                 "X86" CACHE INTERNAL "")
SET(LN_MCU                  "M4" CACHE INTERNAL "")
SET(LN_TOOLCHAIN_EXT        "arm_gd32fx_clang" CACHE INTERNAL "")
SET(LN_ENABLE_RUST          ON   CACHE INTERNAL "")
set(AF_FOLDER               ${CMAKE_SOURCE_DIR}/../lnArduino CACHE INTERNAL "")
set(CMAKE_MODULE_PATH       ${CMAKE_MODULE_PATH} ${CMAKE_SOURCE_DIR}/../cmake ${AF_FOLDER}/cmake CACHE INTERNAL "")


SET(Rust_CARGO_TARGET "" CACHE INTERNAL "")
SET(CMAKE_C_COMPILER /usr/bin/clang CACHE INTERNAL "")
SET(CMAKE_CXX_COMPILER /usr/bin/clang++ CACHE INTERNAL "")
SET(CMAKE_CXX_AR /usr/bin/llvm-ar  CACHE INTERNAL "")
SET(CMAKE_CXX_RANLIB /usr/bin/llvm-ranlib  CACHE INTERNAL "")
#
SET(LN_ENABLED_BOARDS   CH32F1 RP STM32F1 CACHE INTERNAL "")
#
include(swindle_version)
GEN_VERSION_FILE()
#
SET(BMP_VERSION_BRANCH 1.10)
SET(BMP_PATCH_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/../patches_${BMP_VERSION_BRANCH})
SET(LNBMP_TOP_FOLDER ${CMAKE_SOURCE_DIR}/../blackmagic)
include(patch_blackmagic)
#
#

add_library(lnArduino INTERFACE)

target_include_directories(lnArduino INTERFACE ${AF_FOLDER}/)
target_include_directories(lnArduino INTERFACE ${AF_FOLDER}/include)
target_include_directories(lnArduino INTERFACE ${AF_FOLDER}/arduinoLayer/include)
target_include_directories(lnArduino INTERFACE ${AF_FOLDER}/FreeRTOS/include)
target_include_directories(lnArduino INTERFACE ${LN_MCU_FOLDER})
target_include_directories(lnArduino INTERFACE ${LN_MCU_FOLDER}/boards/bluepill)
target_include_directories(lnArduino INTERFACE ${AF_FOLDER}/FreeRTOS/portable/GCC/ARM_CM3/)
target_include_directories(lnArduino INTERFACE ${AF_FOLDER}/mcus/arm_gd32fx/include)

target_include_directories(lnArduino INTERFACE ${CMAKE_BINARY_DIR})



# Qt5

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)


#find_package(Qt5 COMPONENTS Network SerialPort REQUIRED)
find_package(Qt6 COMPONENTS Network SerialPort REQUIRED)


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
# If HOSTED_BMP_ONLY is set, whclink will not work
ADD_DEFINITIONS("-DHOSTED_BMP_ONLY=1") # HOSTED_BMP_ONLY
ADD_DEFINITIONS("-DENABLE_DEBUG=1")
ADD_DEFINITIONS("-g3")
ADD_DEFINITIONS("-DENABLE_DEBUG=1")
ADD_DEFINITIONS("-DBMD_IS_STDC=1")

include(enabled_board)

ADD_SUBDIRECTORY(../swindle lnswindle)

#
#include(rnCmake)
#corrosion_import_crate(MANIFEST_PATH ../lnBlackmagic/rs/lnbmp/Cargo.toml    NO_DEFAULT_FEATURES      CFLAGS ${LN_LTO_RUST_FLAGS} )
#corrosion_add_target_rustflags( rsbmp --cfg feature="hosted")

SET(H ${CMAKE_CURRENT_SOURCE_DIR}/src)
SET(SRC ${H}/main.cpp ${H}/lnDebug.cpp ${H}/stubs.cpp ${H}/bmp_qtnetwork.h ${H}/bmp_qtserial.cpp ${H}/bmp_qtnetwork.cpp)
#${H}/bmp_remote_rv.cpp)

ADD_EXECUTABLE(swindle_hosted ${SRC})
target_link_libraries(swindle_hosted PRIVATE rsbmp)
target_link_libraries(swindle_hosted PRIVATE swindle_interface)
target_link_libraries(swindle_hosted PRIVATE libswindle  )
target_link_libraries(swindle_hosted PRIVATE swindle_interface)
target_link_libraries(swindle_hosted PRIVATE libswindle  )
target_link_libraries(swindle_hosted PRIVATE embeddedPrintf usb-1.0 )
target_link_libraries(swindle_hosted PRIVATE Qt6::Network Qt6::SerialPort)
