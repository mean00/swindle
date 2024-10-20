
include(applyPatch)
#APPLY_PATCH_IF_NEEDED3(patched6 ${BMP_PATCH_FOLDER}/blackmagic_small_exception.patch             ${LNBMP_TOP_FOLDER}   "use smaller setjmp/long jmp on smaller arm")
#APPLY_PATCH_IF_NEEDED3(patched16 ${BMP_PATCH_FOLDER}/blackmagic_make_ebreak_work.patch           ${LNBMP_TOP_FOLDER}   "riscv make sw breakpoint work")
#APPLY_PATCH_IF_NEEDED3(patched18 ${BMP_PATCH_FOLDER}/blackmagic_riscv_stub.patch                 ${LNBMP_TOP_FOLDER}   "declare flashstub function")

APPLY_PATCH_IF_NEEDED3(patched8 ${BMP_PATCH_FOLDER}/blackmagic_no_debug_bmp.patch                ${LNBMP_TOP_FOLDER}   "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED3(patched7 ${BMP_PATCH_FOLDER}/blackmagic_use_custom_target_mem_map.patch   ${LNBMP_TOP_FOLDER}   "use heap based mem map")
APPLY_PATCH_IF_NEEDED3(patched9 ${BMP_PATCH_FOLDER}/blackmagic_no_libopencm3.patch               ${LNBMP_TOP_FOLDER}   "dont use opencm3 ")
APPLY_PATCH_IF_NEEDED3(patched10 ${BMP_PATCH_FOLDER}/blackmagic_make_swd_scan_public.patch       ${LNBMP_TOP_FOLDER}   "make swdp_scan public ")
APPLY_PATCH_IF_NEEDED3(patched11 ${BMP_PATCH_FOLDER}/blackmagic_use_embedded_printf.patch        ${LNBMP_TOP_FOLDER}   "use embedded printf ")
APPLY_PATCH_IF_NEEDED3(patched12 ${BMP_PATCH_FOLDER}/blackmagic_disable_cortexr.patch            ${LNBMP_TOP_FOLDER}   "disable cortexr ")
APPLY_PATCH_IF_NEEDED3(patched14 ${BMP_PATCH_FOLDER}/blackmagic_disable_snprintf_define.patch    ${LNBMP_TOP_FOLDER}   "disable snprintf define")
APPLY_PATCH_IF_NEEDED3(patched13 ${BMP_PATCH_FOLDER}/blackmagic_rvswd_perigoso.patch             ${LNBMP_TOP_FOLDER}   "add rvswd support (based on perigoso work)")
APPLY_PATCH_IF_NEEDED3(patched17 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")
APPLY_PATCH_IF_NEEDED3(patched21 ${BMP_PATCH_FOLDER}/blackmagic_riscv_flashstub_vanilla.patch    ${LNBMP_TOP_FOLDER}   "declare flashstub function")
APPLY_PATCH_IF_NEEDED3(patched25 ${BMP_PATCH_FOLDER}/blackmagic_riscv_disable_interrupt_during_step.patch ${LNBMP_TOP_FOLDER}   "disable interrupt during single step")
APPLY_PATCH_IF_NEEDED3(patched26 ${BMP_PATCH_FOLDER}/blackmagic_riscv_check_watchpoint.patch ${LNBMP_TOP_FOLDER}   "explictely fail when we cant put watchpoints")
APPLY_PATCH_IF_NEEDED3(patched32 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
APPLY_PATCH_IF_NEEDED3(patched33 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2_part2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
APPLY_PATCH_IF_NEEDED3(patched34 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning2.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")


