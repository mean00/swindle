#!/bin/sh
#set -x
#
# Note: the checked-in rp2040_crc32.stub was produced with an older arm-none-eabi-gcc
# (/arm/113); rebuilding it with the toolchain below yields a slightly different (but
# equivalent) image. Only replace the checked-in file after validating on hardware.
#
export TOOLCHAIN=/arm/15.2.1/bin
CC=${TOOLCHAIN}/arm-none-eabi-gcc
OBJCOPY=${TOOLCHAIN}/arm-none-eabi-objcopy
OBJDUMP=${TOOLCHAIN}/arm-none-eabi-objdump
OPT="-mcpu=cortex-m0plus"

SYS=${TOOLCHAIN}/../arm/llvm_21/lib/clang-runtimes/arm-none-eabi/armv6m_soft_nofp/include
build_stub() {
  rm -f $1.o
  set -x
  $CC $1.c -g -O2 -c ${OPT} -o $1.o -nostdlib -I ../../blackmagic/src/target -I ../../blackmagic/src/include -I ${SYS}
  $OBJCOPY -Obinary $1.o $1.bin
  $OBJDUMP -S $1.o >$1.asm
  xxd -i $1.bin >$1.tmp
  cat $1.tmp | sed 's/unsigned char/static const unsigned char/g' | sed 's/^.*_len =/#define RP2040_CRC32_BIN_LEN/g' | head -c -2 >$1.stub
}

build_stub rp2040_crc32
