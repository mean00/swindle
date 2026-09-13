export TOOLPATH=/riscv/xpack-14.2.0-2/bin/riscv-none-elf-

fail() {
  echo "*** FAIL ** $1"
  exit -1
}

gen() {
  echo "Generating $1"
  export NAME=$1

  ${TOOLPATH}gcc \
    -g \
    -march=rv32ec_zicsr -mabi=ilp32e \
    -O3 -ffreestanding -nostdlib -fomit-frame-pointer \
    -I../../../flashstub \
    $1.c -o $1.o \
    -c || fail gcc
  ${TOOLPATH}objcopy -Obinary $1.o $1.bin
  ${TOOLPATH}objdump -Sd $1.o >$1.asm
  xxd -i $1.bin >$1.h1
  cat $1.h1 | sed 's/unsigned char/const unsigned char/g' | head -c -1 >$1.stub
  #cat $1.h1 | sed 's/unsigned char/const unsigned char/g' | sed 's/^.*_len =/#define $1/g' | head -c -1 >$1.stub
}

gen ch32v0x_write

echo "Done."
