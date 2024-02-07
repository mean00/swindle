CC=/riscv/tools_llvm/bin/clang-17
OBJCOPY=/riscv/tools_llvm/bin/llvm-objcopy
OBJDUMP=/riscv/tools_llvm/bin/llvm-objdump

build_stub() {
	rm -f $1.o
	set -x
	$CC $1.c -g -O2 -o $1.o -nostdlib
	$OBJCOPY -Obinary $1.o $1.bin
	$OBJDUMP -S $1.o > $1.asm
	xxd -i $1.bin >$1.stub
}

build_stub ch32v3x_erase
build_stub ch32v3x_write
