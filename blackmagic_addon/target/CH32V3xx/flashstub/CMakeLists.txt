cmake_minimum_required(VERSION 3.20)

get_filename_component(swindle "${CMAKE_CURRENT_SOURCE_DIR}/../../../../" ABSOLUTE)
get_filename_component(blackmagic "${CMAKE_CURRENT_SOURCE_DIR}/../../../../blackmagic" ABSOLUTE)

message(STATUS "Swindle    : ${swindle}")
message(STATUS "Blackmagic : ${blackmagic}")

set(LN_ARCH "RISCV")
set(LN_MCU "CH32V3x")

#
# reuse the compiler from the main project
#
include(${swindle}/platformConfig.cmake)
#
set(CMAKE_SYSTEM_NAME
    Generic
    CACHE INTERNAL "")
set(CMAKE_C_COMPILER_ID
    "Clang"
    CACHE INTERNAL "")
set(CMAKE_CXX_COMPILER_ID
    "Clang"
    CACHE INTERNAL "")
set(CMAKE_C_COMPILER_WORKS
    TRUE
    CACHE INTERNAL "")
set(CMAKE_CXX_COMPILER_WORKS
    TRUE
    CACHE INTERNAL "")

set(CMAKE_C_COMPILER
    ${PLATFORM_CLANG_PATH}/clang${PLATFORM_CLANG_VERSION}${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_ASM_COMPILER
    ${PLATFORM_CLANG_PATH}/clang${PLATFORM_CLANG_VERSION}${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_CXX_COMPILER
    ${PLATFORM_CLANG_PATH}/clang++${PLATFORM_CLANG_VERSION}${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_EXECUTABLE_SUFFIX_C
    .elf
    CACHE INTERNAL "")
set(CMAKE_EXECUTABLE_SUFFIX_CXX
    .elf
    CACHE INTERNAL "")
set(CMAKE_C_FLAGS
    ""
    CACHE INTERNAL "")
set(CMAKE_C_LINK_EXECUTABLE
    "<CMAKE_CXX_COMPILER> -ffreestanding -nostartfiles   ${PLATFORM_CLANG_EXTRA_LD_ARG} <CMAKE_CXX_LINK_FLAGS>  ${PLATFORM_CLANG_C_FLAGS} <LINK_FLAGS>   -Wl,--start-group  <OBJECTS> <LINK_LIBRARIES> -Wl,--end-group  -Wl,-Map,<TARGET>.map   -o <TARGET> ${GD32_LD_FLAGS} ${GD32_LD_LIBS} ${GD32_LD_FLAGS} --sysroot=${PLATFORM_CLANG_SYSROOT} -I${PLATFORM_CLANG_C_FLAGS}  "
    CACHE INTERNAL "")
# set(CMAKE_CXX_LINK_EXECUTABLE    "<CMAKE_CXX_COMPILER> -ffreestanding -nostartfiles   ${PLATFORM_CLANG_EXTRA_LD_ARG}
# <CMAKE_CXX_LINK_FLAGS>  ${PLATFORM_CLANG_C_FLAGS} <LINK_FLAGS>   -Wl,--start-group  <OBJECTS> <LINK_LIBRARIES>
# -Wl,--end-group  -Wl,-Map,<TARGET>.map   -o <TARGET> ${GD32_LD_FLAGS} ${GD32_LD_LIBS} ${GD32_LD_FLAGS}
# --sysroot=${PLATFORM_CLANG_SYSROOT} -I${PLATFORM_CLANG_C_FLAGS}  "  CACHE INTERNAL "") set(CMAKE_CXX_LINK_EXECUTABLE
# "echo Fake_linking  "  CACHE INTERNAL "") set(CMAKE_C_LINK_EXECUTABLE    "echo Fake_linking  "  CACHE INTERNAL "")
set(CMAKE_SIZE
    ${PLATFORM_CLANG_PATH}/llvm-size${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_AR
    ${PLATFORM_CLANG_PATH}/llvm-ar${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_RANLIB
    ${PLATFORM_CLANG_PATH}/llvm-ranlib${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_OBJCOPY
    ${PLATFORM_CLANG_PATH}/llvm-objcopy${TOOLCHAIN_SUFFIX}
    CACHE PATH "" FORCE)
set(CMAKE_TRY_COMPILE_TARGET_TYPE
    STATIC_LIBRARY
    CACHE INTERNAL "")

set(GD32_MCU_C_FLAGS
    "--sysroot ${PLATFORM_CLANG_SYSROOT} ${EXTRA_DEBUG} ${PLATFORM_CLANG_C_FLAGS} -DLN_MCU=LN_MCU_CH32V3x -DLN_ARCH=LN_ARCH_RISCV ${LN_BOARD_NAME_FLAG} -I${ESPRIT_ROOT}/riscv_ch32v3x/"
    CACHE INTERNAL "")
set(GD32_C_FLAGS
    "-DLN_MCU_XTAL_CLOCK=${LN_MCU_XTAL_CLOCK} ${GD32_SPECS_SPECS} ${GD32_MCU_C_FLAGS} ${GD32_DEBUG_FLAGS}  -Werror=return-type  -fmessage-length=0 -fsigned-char -ffunction-sections -fdata-sections -fno-common "
    CACHE INTERNAL "")
set(CMAKE_C_FLAGS
    "${GD32_C_FLAGS} -O2"
    CACHE INTERNAL "")
set(CMAKE_ASM_FLAGS
    "${GD32_C_FLAGS}"
    CACHE INTERNAL "")
set(CMAKE_CXX_FLAGS
    "${GD32_C_FLAGS}  -fno-rtti -fno-exceptions -fno-threadsafe-statics"
    CACHE INTERNAL "")

project(ch32vxx_flashstub)

macro(gen src)
  add_executable(${src} ${src}.c)
  # elf ->bin
  add_custom_command(
    TARGET ${src}
    POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -O binary $<TARGET_FILE:${src}> $<TARGET_FILE_DIR:${src}>/${src}.bin
    COMMAND ${CMAKE_COMMAND} -E echo "Stub ${src} size:"
    COMMAND du -b ${src}.bin
    COMMAND xxd -i -n ${src}_bin ${src}.bin | sed 's/unsigned char/const uint8_t/g' | sed
            's/unsigned int/const uint32_t/g' > ../${src}.stub
    COMMENT "Extracting raw binary from ELF: ${src}.bin and stub")
endmacro()

include_directories(${PLATFORM_CLANG_PATH}/../lib/clang-runtimes/riscv32-unknown-elf/riscv32_soft_nofp/include)

include_directories(${blackmagic}/src/target/)
include_directories(${blackmagic}/src/include/)
include_directories(${swindle}/blackmagic_addon/flashstub/)
include_directories(${swindle}/swindle/include/)
include_directories(${swindle}/esprit//)
include_directories(${swindle}/esprit/embedded_printf/)

gen(ch32v3x_write)
gen(ch32v3x_erase)
gen(ch32v3x_crc32)
