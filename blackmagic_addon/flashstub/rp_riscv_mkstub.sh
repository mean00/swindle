#!/bin/sh
#
# Regenerate rp_crc32_riscv.stub from rp_crc32_riscv.c.
#
# Same recipe as the CH32V stubs (see target/rs_ch32v123xx/flashstub): link the
# freestanding object with the RISC-V clang; the stub is position independent (it only
# touches absolute peripheral addresses), so the raw .binary can be dropped anywhere.
#
set -x
TOOLCHAIN=/riscv/llvm_21.1.8/bin
CC=${TOOLCHAIN}/clang-21
OBJCOPY=${TOOLCHAIN}/llvm-objcopy
OBJDUMP=${TOOLCHAIN}/llvm-objdump
SYS=${TOOLCHAIN}/../lib/clang-runtimes/riscv32-unknown-elf/riscv32_soft_nofp
OPT="--target=riscv32 -march=rv32imac_zicsr -mabi=ilp32 --sysroot=${SYS}"

build_stub() {
  rm -f $1.o $1.elf $1.bin
  $CC $1.c -O2 -c -ffreestanding -fno-common ${OPT} -I . -I ../../blackmagic/src/target \
      -I ../../blackmagic/src/include -o $1.o || exit 1
  $CC $1.o -ffreestanding -nostartfiles ${OPT} -o $1.elf || exit 1
  $OBJCOPY -Obinary $1.elf $1.bin
  $OBJDUMP -d $1.elf >$1.asm
  xxd -i $1.bin >$1.tmp
  cat $1.tmp | sed 's/unsigned char/static const unsigned char/g' | \
      sed 's/^.*_len =/#define RP_CRC32_RISCV_BIN_LEN/g' | head -c -2 >$1.stub
  rm -f $1.tmp
}

build_stub rp_crc32_riscv
