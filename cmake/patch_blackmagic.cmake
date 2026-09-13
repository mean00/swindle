# CUSTOM VERSION
include(FindPatch)
message(STATUS "Checking for blackmagic patches : ")
macro(APPLY_PATCH_IF_NEEDED4 markerFile absPatchFile absSubdir description)
  if(NOT EXISTS "${absSubdir}/${markerFile}")
    # MESSAGE(STATUS "   Patching file in ${subdir} ${description}") MESSAGE(STATUS "      patch_file_p(1:::
    # ${absSubdir} <= ${absPatchFile}")
    message(STATUS "patching ${markerFile}  : ${description}")
    patch_file_p(1 "${absSubdir}" "${absPatchFile}")
    file(WRITE "${absSubdir}/${markerFile}" "patched")
  else()
    list(APPEND already_patched "${markerFile} ")
  endif()
endmacro()
#
macro(APPLY_PATCH_IF_NEEDED5 markerFile relPath description)
  apply_patch_if_needed4(${markerFile} ${BMP_PATCH_FOLDER}/${relPath} ${LNBMP_TOP_FOLDER} "${description}")
endmacro()

#
apply_patch_if_needed5(patched08 08blackmagic_no_debug_bmp.patch "undefined bmp_debug removal")
apply_patch_if_needed5(patched07 07blackmagic_use_custom_target_mem_map.patch "use heap based mem map")
apply_patch_if_needed5(patched10 10blackmagic_make_swd_scan_public.patch "make swdp_scan public ")
apply_patch_if_needed5(patched11 11blackmagic_use_embedded_printf.patch "use embedded printf ")
apply_patch_if_needed5(patched12 12blackmagic_disable_cortexr.patch "disable cortexr ")
#
apply_patch_if_needed5(patched14 14blackmagic_disable_snprintf_define.patch "disable snprintf define")
apply_patch_if_needed5(patched13 13blackmagic_rvswd_perigoso.patch "add rvswd support (based on perigoso work)")

apply_patch_if_needed5(patched21 21blackmagic_riscv_flashstub_vanilla.patch "declare flashstub function")

apply_patch_if_needed5(patched26 26blackmagic_riscv_check_watchpoint.patch
                       "explicitely fail when we cant put watchpoints")
# IF(NOT LN_DISABLE_SWINDLE_LNIO)
apply_patch_if_needed5(patched33 33blackmagic_redirect_adiv5_to_ln_v3.patch
                       "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions")
# ENDIF()
apply_patch_if_needed5(patched36 36blackmagic_remove_printf_warning_v3.patch
                       "remove a warning because we ovveride printf")
apply_patch_if_needed5(patched37 37blackmagic_hack_remote_protocolv3.patch
                       "dirty hack to bring back remote protocol v3, probably not the right way")
# Merged with 51 apply_patch_if_needed5(patched40 40blackmagic_custom_crc.patch "compute CRC via a stub.")
#
apply_patch_if_needed5(patched42 42blackmagic_psplim_msplim.patch "add psplim and msplim.")
# Merged with 42 ====>apply_patch_if_needed5(patched43 43dontdoubleassert.patch "Try to manage double assert better.")
apply_patch_if_needed5(patched44 44blackmagic_gd32_crc32.patch "stub CRC32 computation (GD32).")
# Merged with 42 apply_patch_if_needed5(patched45 45blackmagic_disable_bmda_read_regs.patch
#                       "disable shortcut for read/write register in hosted mode.")

apply_patch_if_needed5(patched46 46blackmagic_rp2040_crc32.patch "use RP2040 DMA to compute GDB CRC32")
apply_patch_if_needed5(patched47 47blackmagic_tweak_options.patch "patch option field after probe")
apply_patch_if_needed5(patched48 48blackmagic_enable_native_rvswd.patch "enable rvswd for native mode")
apply_patch_if_needed5(patched49 49blackmagic_hook_rvswd_to_host_mode.patch "enable rvswd for hosted mode")
#partially merged with 42
apply_patch_if_needed5(patched50 50blackmagic_export_breakpoint_available.patch
                       "export a helper function to query the # of hw breakoint/watchpoint")
apply_patch_if_needed5(patched51 51blackmagic_add_sw_breakpoint_framework.patch
                       "hook in the data needed to attach sw breakpoints to target")
apply_patch_if_needed5(patched53 53blackmagic_abort_on_flash_error.patch
                       "abort buffered flash write as soon as there is one error")

apply_patch_if_needed5(patched55 55blackmagic_variable_erase_size.patch
                       "use variable size erase block to speed up erasee on ch32vxx")
#
apply_patch_if_needed5(patched56 56blackmagic_fix_rp2320_flash_cache.patch "Fix aligned check for cache flush (RP2350) ")
apply_patch_if_needed5(patched57 57blackmagic_run_flashtub_witout_irq.patch "When flashing through flashstub, disable interrupt, that helps the RP2040 ")
apply_patch_if_needed5(patched58 58blackmagic_rp2040_core1_reset.patch "Hold RP2040 core 1 in reset while programming flash (PSM FRCE_OFF.PROC1)")
# merged with another patch apply_patch_if_needed5(patched54 54blackmagic_dont_doubly_define_rvswd.patch
#"avoid doubly definit have_rvswd in hosted mode")
apply_patch_if_needed5(patched82 82bisblackmagic_export_arch.patch "Export the cpu arch (ARM/RISCV) ")
apply_patch_if_needed5(patched83bis 83bisblackmagic_select_csw_cache_invalidate_on_reset.patch "cache DP-SELECT/AP-CSW writes (skip unchanged) + invalidate caches on reset")
apply_patch_if_needed5(patched85 85bisblackmagic_riscv_optimizations.patch "RV optimizations (sysbus, progbuf, nostop, benchmark, dmi counters) + shared RV_DM_ABSTRACTAUTO/RV_ABSTRACTAUTO_AUTOEXECDATA_0 and RV_HART_FLAG_MEMORY_ABSTAUTO in riscv_debug.h")
apply_patch_if_needed5(patched87bis 87bisblackmagic_rp2040_spi_robust.patch "spi robustness: abort on target error in RP2040 QSPI flash put/get and generic SPI flash busy-wait loops")
apply_patch_if_needed5(patched88 88blackmagic_adiv6_select_csw_cache.patch "cache ADIv6 DP-SELECT1/SELECT/AP-CSW writes (skip unchanged) in adiv5_ap_select + adiv6_ap_reg_write")
apply_patch_if_needed5(patched89 89blackmagic_lean_dmi_accessor.patch "lean ADI DTM DMI path: drive TAR/DRW directly (no per-op RDBUFF flush + CTRL/STAT poll), warm loop = TAR + data")
apply_patch_if_needed5(patched90 90blackmagic_dp_invalidate_caches.patch "split cache invalidation into DP-level (SELECT/SELECT1, covers ADIv5+ADIv6) and AP-level (CSW); invalidate SELECT/SELECT1 on SWD line reset")
apply_patch_if_needed5(patched92 92blackmagic_riscv_reset_no_allreset_poll.patch "riscv_reset: drop RV_DM_STAT_ALL_RESET poll (dmstatus bit 19 = allhavereset, cleared by reset on v0.13 RP2350 -> always 2x500ms timeout); re-activate DM + wait for dmactive instead")
apply_patch_if_needed5(patched93 93blackmagic_riscv32_rv32e_reg_access.patch "riscv32 single register access: derive the GPR count from the hart ISA (E base has 16 GPRs) so GDB's PC regno is not mis-decoded as GPR x16 -> fixes 'load'/'set $pc' failing with E01 on RV32E (CH32V003)")
apply_patch_if_needed5(patched94 94blackmagic_riscv_command_timeout.patch "add a 200ms timeout to riscv_command_wait_complete to prevent infinite loops when target is disconnected")
apply_patch_if_needed5(patched95 95blackmagic_riscv32_run_stub_rv32e_pc.patch "Write the correct register (PC) when running on RV32E")




string(JOIN " " pretty ${already_patched})
message(STATUS "Patch already applied ${pretty} already done")
# APPLY_PATCH_IF_NEEDED5(patched80 81blackamgic_support_faults_on_cm33.patch       "Properly managed faults on armv8")
# APPLY_PATCH_IF_NEEDED5(patched52 52blackmagic_abort_on_flash_error.patch         "abort buffered flash write as soon
# as there is one error") APPLY_PATCH_IF_NEEDED3(patched80
# ${BMP_PATCH_FOLDER}/80blackmagic_support_watchpoint_on_cm33.patch ${LNBMP_TOP_FOLDER}   "fix cortemx v8m watchpoint
# .") >>APPLY_PATCH_IF_NEEDED3(patched25 ${BMP_PATCH_FOLDER}/25blackmagic_riscv_disable_interrupt_during_step.patch
# ${LNBMP_TOP_FOLDER}   "disable interrupt during single step") Obsoleted by newer version
# APPLY_PATCH_IF_NEEDED3(patched32 ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2.patch ${LNBMP_TOP_FOLDER}
# "redirect adiv5_xx_no_check to ln_adiv5xxx_no_check swd functions") APPLY_PATCH_IF_NEEDED3(patched33
# ${BMP_PATCH_FOLDER}/blackmagic_redirect_adiv5_to_lnv2_part2.patch ${LNBMP_TOP_FOLDER}   "redirect adiv5_xx_no_check to
# ln_adiv5xxx_no_check swd functions") APPLY_PATCH_IF_NEEDED3(patched34
# ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning2.patch      ${LNBMP_TOP_FOLDER}   "remove a warning because we
# ovveride printf") APPLY_PATCH_IF_NEEDED3(patched35 ${BMP_PATCH_FOLDER}/blackmagic_remove_printf_warning.patch
# ${LNBMP_TOP_FOLDER}   "remove a warning because we ovveride printf")
# SELECT/CSW caching for the ARM AP/DP hot path (skip unchanged DP-SELECT and AP-CSW writes)
