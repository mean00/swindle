
include(applyPatch)
APPLY_PATCH_IF_NEEDED2(patched6 ${BMP_PATCH_FOLDER}/blackmagic_small_exception.patch             ${BMP_PREFIX}blackmagic   "use smaller setjmp/long jmp on smaller arm")
APPLY_PATCH_IF_NEEDED2(patched8 ${BMP_PATCH_FOLDER}/blackmagic_no_debug_bmp.patch                ${BMP_PREFIX}blackmagic   "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED2(patched7 ${BMP_PATCH_FOLDER}/blackmagic_use_custom_target_mem_map.patch   ${BMP_PREFIX}blackmagic   "use heap based mem map")
APPLY_PATCH_IF_NEEDED2(patched9 ${BMP_PATCH_FOLDER}/blackmagic_no_libopencm3.patch               ${BMP_PREFIX}blackmagic   "dont use opencm3 ")
APPLY_PATCH_IF_NEEDED2(patched10 ${BMP_PATCH_FOLDER}/blackmagic_make_swd_scan_public.patch       ${BMP_PREFIX}blackmagic   "make swdp_scan public ")
APPLY_PATCH_IF_NEEDED2(patched11 ${BMP_PATCH_FOLDER}/blackmagic_use_embedded_printf.patch        ${BMP_PREFIX}blackmagic   "use embedded printf ")
APPLY_PATCH_IF_NEEDED2(patched12 ${BMP_PATCH_FOLDER}/blackmagic_disable_cortexr.patch            ${BMP_PREFIX}blackmagic   "disable cortexr ")
#APPLY_PATCH_IF_NEEDED2(patched13 ${BMP_PATCH_FOLDER}/blackmagic_rswd_support.patch               ${BMP_PREFIX}blackmagic   "add rvswd support (perigoso)")
#APPLY_PATCH_IF_NEEDED2(patched14 ${BMP_PATCH_FOLDER}/blackmagic_rswd_support2.patch               ${BMP_PREFIX}blackmagic   "add rvswd support  command(perigoso)")

