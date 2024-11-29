set -x
export TOOLCHAIN=/riscv/llvm_19.1/bin/
CC=${TOOLCHAIN}/clang-19
OBJCOPY=${TOOLCHAIN}/llvm-objcopy
OBJDUMP=${TOOLCHAIN}/llvm-objdump
SYS=${TOOLCHAIN}/../lib/clang-runtimes/riscv32-unknown-elf/riscv32_hard_fp/include
build_stub() {
  rm -f $1.o
  set -x
  $CC $1.c -g -O2 -o $1.o -nostdlib -I ../../blackmagic/src/target -I ../../blackmagic/src/include -I ${SYS}
  $OBJCOPY -Obinary $1.o $1.bin
  $OBJDUMP -S $1.o >$1.asm
  xxd -i $1.bin >$1.stub
}

build_stub ch32v3x_erase
build_stub ch32v3x_write
build_stub ch32v3x_crc32
