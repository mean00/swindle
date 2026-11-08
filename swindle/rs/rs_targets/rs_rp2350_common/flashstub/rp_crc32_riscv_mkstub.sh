#!/bin/sh
#
# Regenerate rp_crc32_riscv.stub from rp_crc32_riscv.c:
#
#   ./rp_crc32_riscv_mkstub.sh
#
# Same recipe as the CH32V stubs (see swindle/rs/rs_targets/rs_ch32v123xx/flashstub): link
# the freestanding object with the RISC-V clang; the stub is position independent (it only
# touches absolute peripheral addresses), so the raw .binary can be dropped anywhere.
#
# The resulting array is what blackmagic/src/target/rp2350.c includes; the Rust drivers use
# the images `CMakeLists.txt` builds from `rp2350_crc32.c`/`rp2350_write.c` instead.
#
# Manual, developer-only step: nothing in the probe build runs it, and it reads only the
# sources shipped in the Rust tree (`swindle/rs/rs_targets/include` and
# `rs_rp_common/src/flashstub`), never the blackmagic tree.
set -eu

TOOLCHAIN=/riscv/llvm_21.1.8/bin
CC=${TOOLCHAIN}/clang-21
OBJCOPY=${TOOLCHAIN}/llvm-objcopy
OBJDUMP=${TOOLCHAIN}/llvm-objdump
SYS=${TOOLCHAIN}/../lib/clang-runtimes/riscv32-unknown-elf/riscv32_soft_nofp
OPT="--target=riscv32 -march=rv32imac_zicsr -mabi=ilp32 --sysroot=${SYS}"

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
INCLUDES="-I ${HERE} -I ${HERE}/../../include -I ${HERE}/../../rs_rp_common/src/flashstub"

build_stub() {
    cd "${HERE}"
    rm -f "$1.o" "$1.elf" "$1.bin" "$1.asm"
    ${CC} "$1.c" -O2 -c -ffreestanding -fno-common ${OPT} ${INCLUDES} -o "$1.o"
    ${CC} "$1.o" -ffreestanding -nostartfiles ${OPT} -o "$1.elf"
    ${OBJCOPY} -Obinary "$1.elf" "$1.bin"
    ${OBJDUMP} -d "$1.elf" >"$1.asm"
    # C array form, the shape blackmagic's `#include` expects.
    xxd -i "$1.bin" | sed 's/unsigned char/static const unsigned char/g' |
        sed 's/^.*_len =/#define RP_CRC32_RISCV_BIN_LEN/g' | head -c -2 >"$1.stub"
    rm -f "$1.o" "$1.elf" "$1.bin"
}

build_stub rp_crc32_riscv
printf '%s\n' "wrote rp_crc32_riscv.stub - rp_crc32_riscv.asm kept for inspection"
