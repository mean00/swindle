cmake_minimum_required(VERSION 3.20)
project(rp2350_flashstub NONE)

get_filename_component(swindle "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../" ABSOLUTE)

# Toolchains
set(ARM_CLANG "/arm/llvm_21.1/bin/clang-21")
set(ARM_OBJCOPY "/arm/llvm_21.1/bin/llvm-objcopy")
set(ARM_FLAGS --target=arm-none-eabi -mcpu=cortex-m33 -mthumb -Os -ffreestanding -nostdlib -fno-builtin -Wall -Wextra)

set(RISCV_CLANG "/riscv/llvm_21.1.8/bin/clang-21")
set(RISCV_OBJCOPY "/riscv/llvm_21.1.8/bin/llvm-objcopy")
set(RISCV_FLAGS --target=riscv32 -march=rv32imac_zicsr -mabi=ilp32 -Os -ffreestanding -nostdlib -fno-builtin -Wall -Wextra)

set(WRITE_SRC_C ${CMAKE_CURRENT_SOURCE_DIR}/rp2350_write.c)
set(WRITE_HEADER ${CMAKE_CURRENT_SOURCE_DIR}/rp2350_qmi.h)

set(CRC_SRC_C ${CMAKE_CURRENT_SOURCE_DIR}/rp2350_crc32.c)
set(CRC_HEADER ${CMAKE_CURRENT_SOURCE_DIR}/rp2350_dma.h)

# ARM Write build
set(ARM_ENTRY ${CMAKE_CURRENT_SOURCE_DIR}/start_arm.S)
set(ARM_LDSCRIPT ${CMAKE_CURRENT_SOURCE_DIR}/rp2350_arm.ld)
set(ARM_ELF ${CMAKE_CURRENT_BINARY_DIR}/rp2350_write_arm.elf)
set(ARM_BIN ${CMAKE_CURRENT_BINARY_DIR}/rp2350_write_arm.bin)

add_custom_command(
    OUTPUT ${ARM_BIN}
    COMMAND ${ARM_CLANG} ${ARM_FLAGS} -I${CMAKE_CURRENT_SOURCE_DIR} -T ${ARM_LDSCRIPT} ${ARM_ENTRY} ${WRITE_SRC_C} -o ${ARM_ELF}
    COMMAND ${ARM_OBJCOPY} -O binary ${ARM_ELF} ${ARM_BIN}
    COMMAND ${CMAKE_COMMAND} -E echo "Stub rp2350_write_arm size:"
    COMMAND du -b ${ARM_BIN}
    DEPENDS ${WRITE_SRC_C} ${WRITE_HEADER} ${ARM_ENTRY} ${ARM_LDSCRIPT}
    COMMENT "Compiling ARM Cortex-M33 rp2350 flashstub"
)
add_custom_target(rp2350_write_arm ALL DEPENDS ${ARM_BIN})

# RISC-V Write build
set(RISCV_ENTRY ${CMAKE_CURRENT_SOURCE_DIR}/start_riscv.S)
set(RISCV_LDSCRIPT ${CMAKE_CURRENT_SOURCE_DIR}/rp2350_riscv.ld)
set(RISCV_ELF ${CMAKE_CURRENT_BINARY_DIR}/rp2350_write_riscv.elf)
set(RISCV_BIN ${CMAKE_CURRENT_BINARY_DIR}/rp2350_write_riscv.bin)

add_custom_command(
    OUTPUT ${RISCV_BIN}
    COMMAND ${RISCV_CLANG} ${RISCV_FLAGS} -I${CMAKE_CURRENT_SOURCE_DIR} -T ${RISCV_LDSCRIPT} ${RISCV_ENTRY} ${WRITE_SRC_C} -o ${RISCV_ELF}
    COMMAND ${RISCV_OBJCOPY} -O binary ${RISCV_ELF} ${RISCV_BIN}
    COMMAND ${CMAKE_COMMAND} -E echo "Stub rp2350_write_riscv size:"
    COMMAND du -b ${RISCV_BIN}
    DEPENDS ${WRITE_SRC_C} ${WRITE_HEADER} ${RISCV_ENTRY} ${RISCV_LDSCRIPT}
    COMMENT "Compiling RISC-V Hazard3 rp2350 flashstub"
)
add_custom_target(rp2350_write_riscv ALL DEPENDS ${RISCV_BIN})

# ARM CRC32 build
set(ARM_CRC_ENTRY ${CMAKE_CURRENT_SOURCE_DIR}/start_crc32_arm.S)
set(ARM_CRC_ELF ${CMAKE_CURRENT_BINARY_DIR}/rp2350_crc32_arm.elf)
set(ARM_CRC_BIN ${CMAKE_CURRENT_BINARY_DIR}/rp2350_crc32_arm.bin)

add_custom_command(
    OUTPUT ${ARM_CRC_BIN}
    COMMAND ${ARM_CLANG} ${ARM_FLAGS} -I${CMAKE_CURRENT_SOURCE_DIR} -T ${ARM_LDSCRIPT} ${ARM_CRC_ENTRY} ${CRC_SRC_C} -o ${ARM_CRC_ELF}
    COMMAND ${ARM_OBJCOPY} -O binary ${ARM_CRC_ELF} ${ARM_CRC_BIN}
    COMMAND ${CMAKE_COMMAND} -E echo "Stub rp2350_crc32_arm size:"
    COMMAND du -b ${ARM_CRC_BIN}
    DEPENDS ${CRC_SRC_C} ${CRC_HEADER} ${ARM_CRC_ENTRY} ${ARM_LDSCRIPT}
    COMMENT "Compiling ARM Cortex-M33 rp2350 crc32 stub"
)
add_custom_target(rp2350_crc32_arm ALL DEPENDS ${ARM_CRC_BIN})

# RISC-V CRC32 build
set(RISCV_CRC_ENTRY ${CMAKE_CURRENT_SOURCE_DIR}/start_crc32_riscv.S)
set(RISCV_CRC_ELF ${CMAKE_CURRENT_BINARY_DIR}/rp2350_crc32_riscv.elf)
set(RISCV_CRC_BIN ${CMAKE_CURRENT_BINARY_DIR}/rp2350_crc32_riscv.bin)

add_custom_command(
    OUTPUT ${RISCV_CRC_BIN}
    COMMAND ${RISCV_CLANG} ${RISCV_FLAGS} -I${CMAKE_CURRENT_SOURCE_DIR} -T ${RISCV_LDSCRIPT} ${RISCV_CRC_ENTRY} ${CRC_SRC_C} -o ${RISCV_CRC_ELF}
    COMMAND ${RISCV_OBJCOPY} -O binary ${RISCV_CRC_ELF} ${RISCV_CRC_BIN}
    COMMAND ${CMAKE_COMMAND} -E echo "Stub rp2350_crc32_riscv size:"
    COMMAND du -b ${RISCV_CRC_BIN}
    DEPENDS ${CRC_SRC_C} ${CRC_HEADER} ${RISCV_CRC_ENTRY} ${RISCV_LDSCRIPT}
    COMMENT "Compiling RISC-V Hazard3 rp2350 crc32 stub"
)
add_custom_target(rp2350_crc32_riscv ALL DEPENDS ${RISCV_CRC_BIN})

# Generate Rust module: swindle/rs/rs_targets/rs_rp2350_common/src/flashstub.rs
set(STUB_RS ${CMAKE_CURRENT_SOURCE_DIR}/../src/flashstub.rs)
set(STUB_RS_BINS_ARG "${ARM_BIN},${RISCV_BIN},${ARM_CRC_BIN},${RISCV_CRC_BIN}")

add_custom_target(
    flashstub_rs ALL
    COMMAND ${CMAKE_COMMAND} "-DOUT=${STUB_RS}" "-DCRATE=rs_rp2350_common"
            "-DREGEN=swindle/rs/rs_targets/rs_rp2350_common/flashstub" "-DBINS=${STUB_RS_BINS_ARG}" -P
            ${CMAKE_CURRENT_SOURCE_DIR}/../../cmake/gen_flashstub_rs.cmake
    DEPENDS ${ARM_BIN} ${RISCV_BIN} ${ARM_CRC_BIN} ${RISCV_CRC_BIN}
    COMMENT "Generating ${STUB_RS}"
)
add_dependencies(flashstub_rs rp2350_write_arm rp2350_write_riscv rp2350_crc32_arm rp2350_crc32_riscv)
