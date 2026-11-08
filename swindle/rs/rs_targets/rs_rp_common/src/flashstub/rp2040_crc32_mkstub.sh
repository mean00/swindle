#!/bin/sh
#
# Build the RP2040 DMA-sniffer CRC32 stub and rewrite its C array form:
#
#   ./rp2040_crc32_mkstub.sh
#
#   rp2040_crc32.c --arm gcc--> rp2040_crc32.bin --> rp2040_crc32.stub (checked in,
#                                                    |                 included by
#                                                    |                 blackmagic's rp2040.c/rp2350.c)
#                                                    +--> ../flashstub_crc32.rs
#                                                         (`flashstub_crc32_mkstub.sh`)
#
# The image is what both RP drivers park in SRAM (`rs_rp2040::crc32` and the Cortex-M33
# half of the RP2350 driver) and read through `run_stub_result()`: checksum in r1,
# `bkpt #1` on success. This script and the sources next to it are self-contained --
# nothing here reads the blackmagic tree.
#
# Note: the checked-in rp2040_crc32.stub was produced with an older arm-none-eabi-gcc; a
# local rebuild yields a slightly different (but equivalent) image. Only replace the
# checked-in file after validating on hardware.
#
# Override the toolchain with `ARM_GCC=/path/to/arm-none-eabi-gcc ./rp2040_crc32_mkstub.sh`.
set -eu

ARM_GCC=${ARM_GCC:-/arm/15.2.1/bin/arm-none-eabi-gcc}
PREFIX=$(dirname "${ARM_GCC}")/arm-none-eabi-
OBJCOPY=${OBJCOPY:-${PREFIX}objcopy}
OBJDUMP=${OBJDUMP:-${PREFIX}objdump}
# The RP2040's Cortex-M0+ is the oldest core that runs this image.
CPU=${CPU:--mcpu=cortex-m0plus}

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SRC=rp2040_crc32
cd "${HERE}"

rm -f "${SRC}.o" "${SRC}.bin" "${SRC}.asm"
"${ARM_GCC}" "${SRC}.c" -g -Os -c -mthumb ${CPU} -ffreestanding -nostdlib \
    -I "${HERE}" -I "${HERE}/../../../include" -o "${SRC}.o"
"${OBJCOPY}" -Obinary "${SRC}.o" "${SRC}.bin"
"${OBJDUMP}" -S "${SRC}.o" >"${SRC}.asm"

# C array form: the shape blackmagic's `#include` expects (a `static const` array plus a
# length macro, no trailing newline).
xxd -i "${SRC}.bin" | sed 's/unsigned char/static const unsigned char/g' |
    sed 's/^.*_len =/#define RP2040_CRC32_BIN_LEN/g' | head -c -2 >"${SRC}.stub"

rm -f "${SRC}.o" "${SRC}.bin"
printf '%s\n' "wrote ${SRC}.stub - ${SRC}.asm kept for inspection"
