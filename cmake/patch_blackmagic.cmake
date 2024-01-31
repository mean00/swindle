
include(applyPatch)
APPLY_PATCH_IF_NEEDED3(patched6 ${BMP_PATCH_FOLDER}/blackmagic_small_exception.patch             ${LNBMP_TOP_FOLDER}   "use smaller setjmp/long jmp on smaller arm")
APPLY_PATCH_IF_NEEDED3(patched8 ${BMP_PATCH_FOLDER}/blackmagic_no_debug_bmp.patch                ${LNBMP_TOP_FOLDER}   "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED3(patched7 ${BMP_PATCH_FOLDER}/blackmagic_use_custom_target_mem_map.patch   ${LNBMP_TOP_FOLDER}   "use heap based mem map")
APPLY_PATCH_IF_NEEDED3(patched9 ${BMP_PATCH_FOLDER}/blackmagic_no_libopencm3.patch               ${LNBMP_TOP_FOLDER}   "dont use opencm3 ")
APPLY_PATCH_IF_NEEDED3(patched10 ${BMP_PATCH_FOLDER}/blackmagic_make_swd_scan_public.patch       ${LNBMP_TOP_FOLDER}   "make swdp_scan public ")
APPLY_PATCH_IF_NEEDED3(patched11 ${BMP_PATCH_FOLDER}/blackmagic_use_embedded_printf.patch        ${LNBMP_TOP_FOLDER}   "use embedded printf ")
APPLY_PATCH_IF_NEEDED3(patched12 ${BMP_PATCH_FOLDER}/blackmagic_disable_cortexr.patch            ${LNBMP_TOP_FOLDER}   "disable cortexr ")
APPLY_PATCH_IF_NEEDED3(patched14 ${BMP_PATCH_FOLDER}/blackmagic_disable_snprintf_define.patch    ${LNBMP_TOP_FOLDER}   "disable snprintf define")
APPLY_PATCH_IF_NEEDED3(patched13 ${BMP_PATCH_FOLDER}/blackmagic_rvswd_perigoso.patch             ${LNBMP_TOP_FOLDER}   "add rvswd support (based on perigoso work)")


