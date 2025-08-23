#!/usr/bin/bash 
export COMMON="-DUSE_NO_DEFAULT=TRUE  -DUSE_CLANG=True   " 
export PATH=/home/jenkins/.cargo/bin:/home/jenkins/.local/bin:$PATH
export PICO_SDK=/opt/pico/pico-sdk
export ROOT=$PWD

runbuild()
{
	local cur_dir=$PWD
	local folder_name="build${1}"
	shift
	rm -Rf $cur_dir/$folder_name
	mkdir $cur_dir/$folder_name
	cd $folder_name
	cmake ${COMMON} $@ ..
	make -j 4
	cd $cur_dir 
}

runbuild G3_WS_64 -DUSE_WS2812=True -DUSE_INVERTED_NRST=True -DUSE_64PIN_PACKAGE=True   -DUSE_GD32F3=True 
runbuild G3_WS_48 -DUSE_WS2812=True -DUSE_INVERTED_NRST=True -DUSE_48PIN_PACKAGE=True   -DUSE_GD32F3=True 
runbuild G3_48    -DUSE_48PIN_PACKAGE=True   -DUSE_GD32F3=True 
runbuild CH32     -DUSE_64PIN_PACKAGE=True   -DUSE_CH32v3x_HW_IRQ_STACK=True -DUSE_HW_FPU=True -DUSE_CH32V3x=True 
runbuild CH32_WS  -DUSE_WS2812=True -DUSE_INVERTED_NRST=True -DUSE_64PIN_PACKAGE=True   -DUSE_CH32v3x_HW_IRQ_STACK=True -DUSE_HW_FPU=True -DUSE_CH32V3x=True 
runbuild RP2040 -DUSE_RP2040=True -DUSE_RP_CARRIER=True -DUSE_RP_ZERO=True 
runbuild RP2040_inv -DUSE_RP2040=True -DUSE_RP_CARRIER=True -DUSE_RP_ZERO=True -DUSE_INVERTED_NRST=True

