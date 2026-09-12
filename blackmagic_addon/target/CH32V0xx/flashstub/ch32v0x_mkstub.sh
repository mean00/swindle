export TOOLPATH=/riscv/xpack-14.2.0-2/bin/riscv-none-elf-

fail() {
  echo "*** FAIL ** $1"
  exit -1
}

gen() {
  echo "Generating $1"
  export NAME=$1

  ${TOOLPATH}gcc \
    -march=rv32ec -mabi=ilp32e \
    $1.S -o $1.o \
    -c || fail gcc
  ${TOOLPATH}objcopy -Obinary $1.o $1.bin
  xxd -i $1.bin >$1.h1
  cat $1.h1 | sed 's/unsigned char/const unsigned char/g' | head -c -2 >$1.stub
  #cat $1.h1 | sed 's/unsigned char/const unsigned char/g' | sed 's/^.*_len =/#define $1/g' | head -c -2 >$1.stub
}

gen ch32v0x_write

echo "Done."
