set -x
export TOOLCHAIN=/riscv/llvm_19.1/bin/
CC=${TOOLCHAIN}/clang-19
OBJCOPY=${TOOLCHAIN}/llvm-objcopy
OBJDUMP=${TOOLCHAIN}/llvm-objdump
GCC=/riscv/xpack-14.2.0-2/bin/riscv-none-elf-gcc
LLD=${TOOLCHAIN}/ld.lld
SYS=${TOOLCHAIN}/../lib/clang-runtimes/riscv32-unknown-elf/rv32imac-zicsr-zifencei_soft_nofp/include
build_stub() {
  rm -f $1.o
  set -x
  $CC $1.c -g -O2 -o $1.o -nostdlib -I ../../blackmagic/src/target -I ../../blackmagic/src/include -I ${SYS} -I$PWD
  $GCC $1.c -o $1.elf -nostdlib -T ch32.ld -I ../../blackmagic/src/target -I ../../blackmagic/src/include -I ${SYS} -I$PWD
  $OBJCOPY -Obinary $1.o $1.bin
  $OBJDUMP -S $1.o >$1.asm
  xxd -i $1.bin >$1.tmp
  cat $1.tmp | sed 's/unsigned char/const unsigned char/g' | head -c -2 >$1.stub
  echo ";" >>$1.stub
}

build_stub ch32v3x_erase
build_stub ch32v3x_write
build_stub ch32v3x_crc32
