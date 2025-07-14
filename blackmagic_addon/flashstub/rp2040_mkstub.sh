#!/bin/sh
#set -x
export TOOLCHAIN=/arm/llvm_21/bin/
CC=${TOOLCHAIN}/clang-21
OBJCOPY=${TOOLCHAIN}/llvm-objcopy
OBJDUMP=${TOOLCHAIN}/llvm-objdump
OPT="--target=armv6m-none-eabi -mcpu=cortex-m0"
export TOOLCHAIN=/usr/bin
CC=${TOOLCHAIN}/arm-none-eabi-gcc
OBJCOPY=${TOOLCHAIN}/arm-none-eabi-objcopy
OBJDUMP=${TOOLCHAIN}/arm-none-eabi-objdump
OPT="-mcpu=cortex-m3"

SYS=${TOOLCHAIN}/../arm/llvm_21/lib/clang-runtimes/arm-none-eabi/armv6m_soft_nofp/include
build_stub() {
  rm -f $1.o
  set -x
  $CC $1.c -g -O2 -c ${OPT} -o $1.o -nostdlib -I ../../blackmagic/src/target -I ../../blackmagic/src/include -I ${SYS}
  $OBJCOPY -Obinary $1.o $1.bin
  $OBJDUMP -S $1.o >$1.asm
  xxd -i $1.bin >$1.tmp
  cat $1.tmp | sed 's/unsigned char/const unsigned char/g' | sed 's/^.*_len =/#define RP_CRC32_BIN_LEN/g' | head -c -2 >$1.stub
}

build_stub rp2040_crc32
