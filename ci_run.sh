#!/usr/bin/bash
export COMMON="-DUSE_NO_DEFAULT=TRUE  -DUSE_CLANG=True   "
export PATH=/home/${USER}/.cargo/bin:/home/${USER}/.local/bin:$PATH
export PICO_SDK=/opt/pico/pico-sdk
export ROOT=$PWD

fail() {
  echo "**** FAILURE ${1} ***"
  exit -1
}

runbuild() {
  local cur_dir=$PWD
  local folder_name="build${1}"
  shift
  rm -Rf $cur_dir/$folder_name
  mkdir $cur_dir/$folder_name
  cd $folder_name
  cmake --preset $@ ..
  make -j 4 || fail ${1}
  cd $cur_dir
}

runbuild rp2040 rp2040
runbuild rp2040_inv rp2040_inv
runbuild rp2350 rp2350
runbuild rp2350_inv rp2350_inv
runbuild gd32f303_48 gd32f303_48
runbuild gd32f303_48_inv_ws2812 gd32f303_48_inv_ws2812
runbuild ch32v307 ch32v307
runbuild ch32v307_inv_ws2812 ch32v307_inv_ws2812
#runbuild ch32v307_net ch32v307_net
