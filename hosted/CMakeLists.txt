# ___________________________________________________________ This builds lnBMP in hosted mode same as normal BMP in
# hosted mode + freeRTOS support ___________________________________________________________

cmake_minimum_required(VERSION 3.13)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)
project(lnBMPHosted C CXX)

# Qt5

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
# set(CMAKE_AUTORCC ON) set(CMAKE_AUTOUIC ON)

# find_package(Qt5 COMPONENTS Network SerialPort REQUIRED)
find_package(
  Qt6
  COMPONENTS Network SerialPort
  REQUIRED)

add_definitions("-DCONFIG_BMDA")
#
# Create a dummy setup to build BMP/lnBMP
#
set(LN_EXT arm_gd32fx)
set(LN_MCU_FOLDER ${CMAKE_SOURCE_DIR}/../esprit/mcus/arm_gd32fx/)
set(SWINDLE_HOSTED
    True
    CACHE INTERNAL "")
set(LN_ARCH
    "X86"
    CACHE INTERNAL "")
set(LN_MCU
    "M4"
    CACHE INTERNAL "")
set(LN_TOOLCHAIN_EXT
    "arm_gd32fx_clang"
    CACHE INTERNAL "")
set(LN_ENABLE_RUST
    ON
    CACHE INTERNAL "")
set(ESPRIT_ROOT
    ${CMAKE_SOURCE_DIR}/../esprit
    CACHE INTERNAL "")
set(CMAKE_MODULE_PATH
    ${CMAKE_MODULE_PATH} ${CMAKE_SOURCE_DIR}/../cmake ${ESPRIT_ROOT}/cmake
    CACHE INTERNAL "")

set(Rust_CARGO_TARGET
    ""
    CACHE INTERNAL "")
# SET(CMAKE_C_COMPILER /usr/bin/clang CACHE INTERNAL "") SET(CMAKE_CXX_COMPILER /usr/bin/clang++ CACHE INTERNAL "")
# SET(CMAKE_CXX_AR /usr/bin/llvm-ar  CACHE INTERNAL "") SET(CMAKE_CXX_RANLIB /usr/bin/llvm-ranlib  CACHE INTERNAL "")
#
set(LN_ENABLED_BOARDS
    CH32F1 RP2350 RP2040 STM32F1
    CACHE INTERNAL "")
#
include(swindle_version)
gen_version_file()
#
set(BMP_VERSION_BRANCH 1.10)
set(BMP_PATCH_FOLDER ${CMAKE_CURRENT_SOURCE_DIR}/../patches_${BMP_VERSION_BRANCH})
set(LNBMP_TOP_FOLDER ${CMAKE_SOURCE_DIR}/../blackmagic)
include(patch_blackmagic)
#

add_library(esprit_dev INTERFACE)
add_library(esprit INTERFACE)

target_include_directories(esprit_dev INTERFACE ${ESPRIT_ROOT}/)
target_include_directories(esprit_dev INTERFACE ${ESPRIT_ROOT}/include)
target_include_directories(esprit_dev INTERFACE ${ESPRIT_ROOT}/FreeRTOS/include)
target_include_directories(esprit_dev INTERFACE ${LN_MCU_FOLDER})
target_include_directories(esprit_dev INTERFACE ${LN_MCU_FOLDER}/boards/bluepill)
target_include_directories(esprit_dev INTERFACE ${ESPRIT_ROOT}/FreeRTOS/portable/GCC/ARM_CM3/)
target_include_directories(esprit_dev INTERFACE ${ESPRIT_ROOT}/mcus/arm_gd32fx/include)
target_include_directories(esprit_dev INTERFACE ${ESPRIT_ROOT}/mcus/common_bluepill/include)

target_include_directories(esprit_dev INTERFACE ${CMAKE_BINARY_DIR})

set(BMP ${CMAKE_SOURCE_DIR}/../blackmagic)
set(ARDUINO_GD32_FREERTOS ${CMAKE_SOURCE_DIR}/../esprit)
add_definitions("-DLN_LINUX=1")
include(${ARDUINO_GD32_FREERTOS}/cmake/lnEmbeddedPrint.cmake)

include_directories(include)
include_directories(${CMAKE_BINARY_DIR})
include_directories(${BMP}/)
include_directories(${BMP}/src/include)
include_directories(${BMP}/src/platforms/pc)
include_directories(${BMP}/src/platforms/hosted)
include_directories(${BMP}/src/target)
include_directories(${BMP}/../lnBlackmagic/include)
#
add_definitions("-DPC_HOSTED=1")
add_definitions("-DLN_NO_FREERTOS")
# If HOSTED_BMP_ONLY is set, whclink will not work
add_definitions("-DHOSTED_BMP_ONLY=1") # HOSTED_BMP_ONLY
add_definitions("-DENABLE_DEBUG=1")
add_definitions("-g3")
add_definitions("-DENABLE_DEBUG=1")
add_definitions("-DBMD_IS_STDC=1")

include(all_boards)
target_include_directories(esprit INTERFACE esprit_dev)

add_subdirectory(../swindle lnswindle)

#
# include(rnCmake) corrosion_import_crate(MANIFEST_PATH ../lnBlackmagic/rs/lnbmp/Cargo.toml    NO_DEFAULT_FEATURES
# CFLAGS ${LN_LTO_RUST_FLAGS} ) corrosion_add_target_rustflags( rsbmp --cfg feature="hosted")

set(H ${CMAKE_CURRENT_SOURCE_DIR}/src)
set(SRC
    ${H}/timeout.cpp
    ${H}/main.cpp
    ${H}/lnDebug.cpp
    ${H}/stubs.cpp
    ${H}/dm_decoder.cpp
    ${H}/bmp_qtserial.cpp
    # ${H}/bmp_qtnetwork.h ${H}/bmp_qtnetwork.cpp
    ${H}/esp_qtnetwork.cpp
    ${H}/esp_qtnetwork.h
    ${H}/esp_bridge.cpp
    ${H}/esp_eventgroup.cpp
    ../../esprit/src/lnSocketRunner.cpp)
# ${H}/bmp_remote_rv.cpp)

add_executable(swindle_hosted ${SRC})
include_directories(../swindle/src/net)
target_compile_definitions(swindle_hosted PRIVATE LN_ARCH=LN_ARCH_ARM)
target_link_libraries(swindle_hosted PRIVATE rsbmp_wrapper)
target_link_libraries(swindle_hosted PRIVATE esprit_dev)
target_link_libraries(swindle_hosted PRIVATE swindle_interface)
target_link_libraries(swindle_hosted PRIVATE libswindle)
target_link_libraries(swindle_hosted PRIVATE swindle_interface)
target_link_libraries(swindle_hosted PRIVATE libswindle)
target_link_libraries(swindle_hosted PRIVATE embeddedPrintf usb-1.0)
target_link_libraries(swindle_hosted PRIVATE Qt6::Core Qt6::Network Qt6::SerialPort)

# Qt6 adds -mno-direct-extern-access which clang rejects
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  foreach(qt_target Qt6::Core Qt6::Network Qt6::SerialPort swindle_hosted)
    get_target_property(qt_iflags ${qt_target} INTERFACE_COMPILE_OPTIONS)
    if(qt_iflags)
      string(REPLACE "-mno-direct-extern-access" "" qt_iflags "${qt_iflags}")
      set_target_properties(${qt_target} PROPERTIES INTERFACE_COMPILE_OPTIONS "${qt_iflags}")
    endif()
  endforeach()
endif()
