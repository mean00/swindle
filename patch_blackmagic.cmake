
include(applyPatch)
APPLY_PATCH_IF_NEEDED(patched6 ${BMP_PREFIX2}blackmagic_small_exception.patch             ${BMP_PREFIX}blackmagic         "use smaller setjmp/long jmp on smaller arm")
APPLY_PATCH_IF_NEEDED(patched8 ${BMP_PREFIX2}blackmagic_no_debug_bmp.patch                ${BMP_PREFIX}blackmagic         "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED(patched7 ${BMP_PREFIX2}blackmagic_use_custom_target_mem_map.patch   ${BMP_PREFIX}blackmagic      "use heap based mem map")
APPLY_PATCH_IF_NEEDED(patched9 ${BMP_PREFIX2}blackmagic_no_libopencm3.patch               ${BMP_PREFIX}blackmagic      "use heap based mem map")
APPLY_PATCH_IF_NEEDED(patched10 ${BMP_PREFIX2}blackmagic_make_swd_scan_public.patch       ${BMP_PREFIX}blackmagic      "make swdp_scan public ")
