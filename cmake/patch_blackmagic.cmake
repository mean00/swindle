
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
MACRO(APPLY_PATCH_IF_NEEDED5 markerFile relPath description)
  APPLY_PATCH_IF_NEEDED4(${markerFile}  ${BMP_PATCH_FOLDER}/${relPath}                 ${LNBMP_TOP_FOLDER}   "${description}")
ENDMACRO()


#
APPLY_PATCH_IF_NEEDED5(patched08 08blackmagic_no_debug_bmp.patch                 "undefined bmp_debug removal")
APPLY_PATCH_IF_NEEDED5(patched07 07blackmagic_use_custom_target_mem_map.patch    "use heap based mem map")
APPLY_PATCH_IF_NEEDED5(patched10 10blackmagic_make_swd_scan_public.patch         "make swdp_scan public ")
APPLY_PATCH_IF_NEEDED5(patched11 11blackmagic_use_embedded_printf.patch          "use embedded printf ")
APPLY_PATCH_IF_NEEDED5(patched12 12blackmagic_disable_cortexr.patch              "disable cortexr ")
APPLY_PATCH_IF_NEEDED5(patched14 14blackmagic_disable_snprintf_define.patch      "disable snprintf define")
APPLY_PATCH_IF_NEEDED5(patched13 13blackmagic_rvswd_perigoso.patch               "add rvswd support (based on perigoso work)")
APPLY_PATCH_IF_NEEDED5(patched21 21blackmagic_riscv_flashstub_vanilla.patch      "declare flashstub function")
APPLY_PATCH_IF_NEEDED5(patched26 26blackmagic_riscv_check_watchpoint.patch       "explicitely fail when we cant put watchpoints")
APPLY_PATCH_IF_NEEDED5(patched33 33blackmagic_redirect_adiv5_to_ln_v3.patch      "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
APPLY_PATCH_IF_NEEDED5(patched36 36blackmagic_remove_printf_warning_v3.patch     "remove a warning because we ovveride printf")
APPLY_PATCH_IF_NEEDED5(patched37 37blackmagic_hack_remote_protocolv3.patch       "dirty hack to bring back remote protocol v3, probably not the right way")
APPLY_PATCH_IF_NEEDED5(patched40 40blackmagic_custom_crc.patch                   "compute CRC via a stub.")
APPLY_PATCH_IF_NEEDED5(patched42 42blackmagic_psplim_msplim.patch                "add psplim and msplim.")
APPLY_PATCH_IF_NEEDED5(patched43 43dontdoubleassert.patch                        "Try to manage double assert better.")
APPLY_PATCH_IF_NEEDED5(patched44 44blackmagic_gd32_crc32.patch                   "stub CRC32 computation (GD32).")
APPLY_PATCH_IF_NEEDED5(patched45 45blackmagic_disable_bmda_read_regs.patch       "disable shortcut for read/write register in hosted mode.")
APPLY_PATCH_IF_NEEDED5(patched46 46blackmagic_rp2040_crc32.patch                 "use RP2040 DMA to compute GDB CRC32")
APPLY_PATCH_IF_NEEDED5(patched47 47blackmagic_tweak_options.patch                "patch option field after probe")
APPLY_PATCH_IF_NEEDED5(patched48 48blackmagic_enable_native_rvswd.patch          "enable rvswd for native mode")
APPLY_PATCH_IF_NEEDED5(patched49 49blackmagic_hook_rvswd_to_host_mode.patch      "enable rvswd for hosted mode")
APPLY_PATCH_IF_NEEDED5(patched50 50blackmagic_export_breakpoint_available.patch  "export a helper function to query the # of hw breakoint/watchpoint")
APPLY_PATCH_IF_NEEDED5(patched51 51blackmagic_add_sw_breakpoint_framework.patch  "hook in the data needed to attach sw breakpoints to target")


string(JOIN " " pretty ${already_patched})
MESSAGE(STATUS "Patch already applied ${pretty} already done")
#APPLY_PATCH_IF_NEEDED3(patched80 ${BMP_PATCH_FOLDER}/80blackmagic_support_watchpoint_on_cm33.patch ${LNBMP_TOP_FOLDER}   "fix cortemx v8m watchpoint .")
#>>APPLY_PATCH_IF_NEEDED3(patched25 ${BMP_PATCH_FOLDER}/25blackmagic_riscv_disable_interrupt_during_step.patch ${LNBMP_TOP_FOLDER}   "disable interrupt during single step")
# Obsoleted by newer version
#APPLY_PATCH_IF_NEEDED3(patched32 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
#APPLY_PATCH_IF_NEEDED3(patched33 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2_part2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
#APPLY_PATCH_IF_NEEDED3(patched34 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning2.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")
#APPLY_PATCH_IF_NEEDED3(patched35 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")



