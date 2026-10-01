#!/bin/bash
set -x
#
gen_cpp() {
  bash ../../../../esprit/cmake/rustgen.sh ${1} ${2} $PWD
}

gen_c() {
  python3 ../../../../esprit/cmake/rustgen.py \
    --lang c \
    --header ${1} \
    --output ${2} \
    --header-in swindle_header.rs.in \
    --include-path ../../../../esprit/mcus/arm_gd32fx/boards/bluepill/ \
    --include-path ../../../../esprit/mcus/arm_gd32fx/include/ \
    --include-path ../../../../esprit/mcus/common_bluepill/ \
    --include-path ../../../../esprit/legacy/boards/bluepill/ \
    --include-path ../../../../esprit/FreeRTOS/portable/GCC/ARM_CM3/ \
    --extra-dir $PWD
}

# Extract the existing doc comments from rn_bmp_cmd_c.rs to use as the header
head -n 25 ../src/rn_bmp_cmd_c.rs | grep -E '^//|#!' > swindle_header.rs.in

gen_c bmp_command.h ../src/rn_bmp_cmd_c.rs

rm swindle_header.rs.in

