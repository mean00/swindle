#!/usr/bin/env bash
# cargo clippy over the swindle Rust workspace, once per board preset of
# CMakePresets.json, with the target triple and the feature list that
# swindle/rs/CMakeLists.txt hands to corrosion (it always builds with
# NO_DEFAULT_FEATURES, so every feature is spelled out here).
#
# usage: ./clippy.sh [preset]      (no argument => all three presets)
set -u

cd "$(dirname "$0")" || exit 1

# Features every preset enables:
#   fake_std        rust_esprit's std shim (swindle/rs/CMakeLists.txt always adds it)
#   native          rs_swindle's embedded half
#   sdi             CMake option SWINDLE_WITH_SDI (default ON)
#   session_arena   CMake option SWINDLE_SESSION_ARENA (default ON)
#   rust_<family>   one per entry of SWINDLE_RUST_ARM_TARGETS (swindle/swindle_common.cmake)
# Not enabled by these presets: rust_logger / logger_uartN (USE_RUST_LOGGER OFF)
# and network (SWINDLE_USE_NETWORK OFF).
BASE_FEATURES="fake_std,native,sdi,session_arena"
BASE_FEATURES+=",rust_stm32f1,rust_stm32f4,rust_stm32g0,rust_rp2040,rust_rp2350"
BASE_FEATURES+=",rust_nrf51,rust_nrf54l,rust_nrf91"

# preset|Rust_CARGO_TARGET|board features
#   platform    rp2040 (rp2xxx backends) or bluepill (GD32/CH32 backends)
#   package     bluepill48 / bluepill64 (USE_48PIN_PACKAGE / USE_64PIN_PACKAGE)
#   led         ws2812 (USE_WS2812, bluepill boards only)
PRESETS=(
  "rp2040_inv|thumbv6m-none-eabi|rp2040"
  "gd32f303_48_inv_ws2812|thumbv7em-none-eabihf|bluepill,bluepill48,ws2812"
  "ch32v307_inv_ws2812|riscv32imafc-unknown-none-elf|bluepill,bluepill64,ws2812"
)

requested=${1:-}
failed=()
ran=0

for preset in "${PRESETS[@]}"; do
  IFS='|' read -r name target board_features <<< "$preset"
  if [ -n "$requested" ] && [ "$requested" != "$name" ]; then
    continue
  fi
  printf '\n=== clippy %s (%s) ===\n' "$name" "$target"
  cargo clippy --workspace --no-default-features \
    --features "${BASE_FEATURES},${board_features}" \
    --target "$target"
  status=$?
  ran=$((ran + 1))
  if [ "$status" -ne 0 ]; then
    failed+=("$name")
  fi
done

if [ "$ran" -eq 0 ]; then
  printf 'unknown preset %s, expected one of:\n' "$requested" >&2
  for preset in "${PRESETS[@]}"; do
    printf '  %s\n' "${preset%%|*}" >&2
  done
  exit 2
fi

printf '\n=== summary ===\n'
if [ "${#failed[@]}" -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[*]}"
  exit 1
fi
echo "all presets passed"

