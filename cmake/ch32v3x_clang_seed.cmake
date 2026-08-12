# ============================================================================
# cmake/ch32v3x_clang_seed.cmake
#
# Shared CH32V3x clang compile-flag seed.
#
# The ch32v307 firmware toolchain (esprit/mcus/riscv_ch32v3x/toolchain_clang.cmake
# - a vendored git submodule) and the standalone flashstub build
# (blackmagic_addon/target/CH32V3xx/flashstub/CMakeLists.txt) used to carry two
# hand-copied copies of the same GD32_MCU_C_FLAGS / GD32_C_FLAGS formula.
# The flashstub now sources this module; when the esprit submodule can be
# changed, its riscv_ch32v3x toolchain should include this module instead of
# its local copy so there is a single seed for the CH32V3x clang flags.
#
# Requires (provided by platformConfig.cmake):
#   PLATFORM_CLANG_SYSROOT, PLATFORM_CLANG_C_FLAGS
# Optional - used verbatim, left empty when unset, exactly like the historical
# hand-written formulas: EXTRA_DEBUG, LN_BOARD_NAME_FLAG, GD32_SPECS_SPECS,
# GD32_DEBUG_FLAGS, LN_MCU_XTAL_CLOCK (mcuSelect.cmake defines it when needed).
# Sets: GD32_MCU_C_FLAGS, GD32_C_FLAGS, CMAKE_C_FLAGS / ASM_FLAGS / CXX_FLAGS.
# ============================================================================
macro(ln_ch32v3x_clang_seed)
  set(GD32_MCU_C_FLAGS
      "--sysroot ${PLATFORM_CLANG_SYSROOT} ${EXTRA_DEBUG} ${PLATFORM_CLANG_C_FLAGS} -DLN_MCU=LN_MCU_CH32V3x -DLN_ARCH=LN_ARCH_RISCV ${LN_BOARD_NAME_FLAG} -I${ESPRIT_ROOT}/riscv_ch32v3x/"
      CACHE INTERNAL "")
  set(GD32_C_FLAGS
      "-DLN_MCU_XTAL_CLOCK=${LN_MCU_XTAL_CLOCK} ${GD32_SPECS_SPECS} ${GD32_MCU_C_FLAGS} ${GD32_DEBUG_FLAGS}  -Werror=return-type  -fmessage-length=0 -fsigned-char -ffunction-sections -fdata-sections -fno-common "
      CACHE INTERNAL "")
  set(CMAKE_C_FLAGS
      "${GD32_C_FLAGS} -O2"
      CACHE INTERNAL "")
  set(CMAKE_ASM_FLAGS
      "${GD32_C_FLAGS}"
      CACHE INTERNAL "")
  set(CMAKE_CXX_FLAGS
      "${GD32_C_FLAGS}  -fno-rtti -fno-exceptions -fno-threadsafe-statics"
      CACHE INTERNAL "")
endmacro()