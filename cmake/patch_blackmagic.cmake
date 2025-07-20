
# CUSTOM VERSION
include(FindPatch)
MESSAGE(STATUS "Checking for blackmagic patches : ")
MACRO(APPLY_PATCH_IF_NEEDED4 markerFile absPatchFile absSubdir description)
  IF(NOT EXISTS "${absSubdir}/${markerFile}")
    #MESSAGE(STATUS "   Patching file in ${subdir} ${description}")
    #MESSAGE(STATUS "      patch_file_p(1::: ${absSubdir} <= ${absPatchFile}")
    MESSAGE(STATUS "patching ${markerFile}  : ${description}")
    patch_file_p(1 "${absSubdir}" "${absPatchFile}")
    file(WRITE "${absSubdir}/${markerFile}" "patched")
  ELSE()
    LIST(APPEND already_patched   "${markerFile} ")
  ENDIF()
ENDMACRO()
#
APPLY_PATCH_IF_NEEDED4(patched08 ${BMP_PATCH_FOLDER}/08blackmagic_no_debug_bmp.patch                ${LNBMP_TOP_FOLDER}   "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED4(patched07 ${BMP_PATCH_FOLDER}/07blackmagic_use_custom_target_mem_map.patch   ${LNBMP_TOP_FOLDER}   "use heap based mem map")
APPLY_PATCH_IF_NEEDED4(patched10 ${BMP_PATCH_FOLDER}/10blackmagic_make_swd_scan_public.patch       ${LNBMP_TOP_FOLDER}   "make swdp_scan public ")
APPLY_PATCH_IF_NEEDED4(patched11 ${BMP_PATCH_FOLDER}/11blackmagic_use_embedded_printf.patch        ${LNBMP_TOP_FOLDER}   "use embedded printf ")
APPLY_PATCH_IF_NEEDED4(patched12 ${BMP_PATCH_FOLDER}/12blackmagic_disable_cortexr.patch            ${LNBMP_TOP_FOLDER}   "disable cortexr ")
APPLY_PATCH_IF_NEEDED4(patched14 ${BMP_PATCH_FOLDER}/14blackmagic_disable_snprintf_define.patch    ${LNBMP_TOP_FOLDER}   "disable snprintf define")
APPLY_PATCH_IF_NEEDED4(patched13 ${BMP_PATCH_FOLDER}/13blackmagic_rvswd_perigoso.patch             ${LNBMP_TOP_FOLDER}   "add rvswd support (based on perigoso work)")
APPLY_PATCH_IF_NEEDED4(patched21 ${BMP_PATCH_FOLDER}/21blackmagic_riscv_flashstub_vanilla.patch    ${LNBMP_TOP_FOLDER}   "declare flashstub function")
APPLY_PATCH_IF_NEEDED4(patched26 ${BMP_PATCH_FOLDER}/26blackmagic_riscv_check_watchpoint.patch ${LNBMP_TOP_FOLDER}   "explicitely fail when we cant put watchpoints")
APPLY_PATCH_IF_NEEDED4(patched33 ${BMP_PATCH_FOLDER}/33blackmagic_redirect_adiv5_to_ln_v3.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
APPLY_PATCH_IF_NEEDED4(patched36 ${BMP_PATCH_FOLDER}/36blackmagic_remove_printf_warning_v3.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")
APPLY_PATCH_IF_NEEDED4(patched37 ${BMP_PATCH_FOLDER}/37blackmagic_hack_remote_protocolv3.patch      ${LNBMP_TOP_FOLDER}   "dirty hack to bring back remote protocol v3, probably not the right way")
APPLY_PATCH_IF_NEEDED4(patched40 ${BMP_PATCH_FOLDER}/40blackmagic_custom_crc.patch ${LNBMP_TOP_FOLDER}   "compute CRC via a stub.")
APPLY_PATCH_IF_NEEDED4(patched42 ${BMP_PATCH_FOLDER}/42blackmagic_psplim_msplim.patch ${LNBMP_TOP_FOLDER}   "add psplim and msplim.")
APPLY_PATCH_IF_NEEDED4(patched43 ${BMP_PATCH_FOLDER}/43dontdoubleassert.patch ${LNBMP_TOP_FOLDER}   "Try to manage double assert better.")
APPLY_PATCH_IF_NEEDED4(patched44 ${BMP_PATCH_FOLDER}/44blackmagic_gd32_crc32.patch ${LNBMP_TOP_FOLDER}   "stub CRC32 computation (GD32).")
APPLY_PATCH_IF_NEEDED4(patched45 ${BMP_PATCH_FOLDER}/45blackmagic_disable_bmda_read_regs.patch ${LNBMP_TOP_FOLDER} "disable shortcut for read/write register in hosted mode.")
APPLY_PATCH_IF_NEEDED4(patched46 ${BMP_PATCH_FOLDER}/46blackmagic_rp2040_crc32.patch ${LNBMP_TOP_FOLDER} "use RP2040 DMA to compute GDB CRC32")


string(JOIN " " pretty ${already_patched})
MESSAGE(STATUS "Patch already applied ${pretty} already done")
#APPLY_PATCH_IF_NEEDED3(patched80 ${BMP_PATCH_FOLDER}/80blackmagic_support_watchpoint_on_cm33.patch ${LNBMP_TOP_FOLDER}   "fix cortemx v8m watchpoint .")
#>>APPLY_PATCH_IF_NEEDED3(patched25 ${BMP_PATCH_FOLDER}/25blackmagic_riscv_disable_interrupt_during_step.patch ${LNBMP_TOP_FOLDER}   "disable interrupt during single step")
# Obsoleted by newer version
#APPLY_PATCH_IF_NEEDED3(patched32 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
#APPLY_PATCH_IF_NEEDED3(patched33 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2_part2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
#APPLY_PATCH_IF_NEEDED3(patched34 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning2.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")
#APPLY_PATCH_IF_NEEDED3(patched35 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")



