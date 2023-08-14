
include(applyPatch)
APPLY_PATCH_IF_NEEDED(patched6 ${BMP_PREFIX2}blackmagic_small_exception.patch             ${BMP_PREFIX}blackmagic         "use smaller setjmp/long jmp on smaller arm")
APPLY_PATCH_IF_NEEDED(patched8 ${BMP_PREFIX2}blackmagic_no_debug_bmp.patch                ${BMP_PREFIX}blackmagic         "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED(patched7 ${BMP_PREFIX2}blackmagic_use_custom_target_mem_map.patch   ${BMP_PREFIX}blackmagic      "use heap based mem map")
APPLY_PATCH_IF_NEEDED(patched9 ${BMP_PREFIX2}blackmagic_no_libopencm3.patch               ${BMP_PREFIX}blackmagic      "use heap based mem map")
#APPLY_PATCH_IF_NEEDED(patched10 ${BMP_PREFIX2}blackmagic_make_swd_scan_public.patch       ${BMP_PREFIX}blackmagic      "make swdp_scan public ")
# xx APPLY_PATCH_IF_NEEDED(patched11 ${BMP_PREFIX2}blackmagic_use_embedded_printf.patch        ${BMP_PREFIX}blackmagic      "use embedded printf instead of bmp newlib ")
#APPLY_PATCH_IF_NEEDED(patched12 ${BMP_PREFIX2}blackmagic_make_riscv32_register_public_add_single_write.patch        ${BMP_PREFIX}blackmagic      "expose riscv32 reg acces, add single write ")
APPLY_PATCH_IF_NEEDED(patched13 ${BMP_PREFIX2}blackmagic_add_rvswd.patch                  ${BMP_PREFIX}blackmagic      "include rvswd.h")
# xx APPLY_PATCH_IF_NEEDED(patched14 ${BMP_PREFIX2}blackmagic_use_embedded_printf2.patch       ${BMP_PREFIX}blackmagic      "use embedded printf (2) ")
APPLY_PATCH_IF_NEEDED(patched15 ${BMP_PREFIX2}blackmagic_enable_ch32v3xx.patch           ${BMP_PREFIX}blackmagic       "enable the ch32v3xx mcu  ")

# ** APPLY_PATCH_IF_NEEDED(patched14 ${BMP_PREFIX2}blackmagic_undefined_debug_bmp.patch        ${BMP_PREFIX}blackmagic      "dont use bmp_debug.h")

#APPLY_PATCH_IF_NEEDED(patched9 ${BMP_PREFIX2}blackmagic_intercept_commands.patch        ${BMP_PREFIX}blackmagic      "intercept gdb commands ")
