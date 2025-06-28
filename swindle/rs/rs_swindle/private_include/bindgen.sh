#!/bin/bash
set -x
#
gen_cpp() {
  bash ../../../../esprit/cmake/rustgen.sh ${1} ${2} $PWD
}

gen_c() {
  bash ../../../../esprit/cmake/rustgen_c.sh ${1} ${2} $PWD
}

gen_c bmp_command.h ../src/rn_bmp_cmd_c.rs
