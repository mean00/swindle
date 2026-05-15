if(WIN32)
  set(PLATFORM_TOOLCHAIN_SUFFIX ".exe")
endif()

if("${LN_ARCH}" STREQUAL "RISCV") # RISCV
  if(${LN_MCU} STREQUAL "CH32V3x") # ----------CH32V307
    # SET(PLATFORM_PREFIX riscv-none-embed-) # MRS toolchain
    if(WIN32)
      set(PLATFORM_TOOLCHAIN_PATH todo_todo) # Use /c/foo or c:\foo depending if you use mingw cmake or win32 cmake
    else()
      # --- GCC ---------
      set(GCC_OPTIM "-msave-restore")
      set(PLATFORM_TOOLCHAIN_PATH
          "/riscv/xpack-14.2.0-2/bin"
          CACHE INTERNAL "")
      if(USE_HW_FPU)
        set(PLATFORM_C_FLAGS
            "-march=rv32imafc_zicsr -mabi=ilp32f ${GCC_OPTIM}"
            CACHE INTERNAL "")
      else()
        set(PLATFORM_C_FLAGS
            "-march=rv32imac_zicsr -mabi=ilp32  ${GCC_OPTIM}"
            CACHE INTERNAL "")
      endif()
      set(PLATFORM_PREFIX
          "riscv-none-elf-"
          CACHE INTERNAL "")
      # FOR CLANG

      set(PLATFORM_TOOLCHAIN_TRIPLET
          "riscv-none-elf-"
          CACHE INTERNAL "")
      # -- CLANG -- No FPU SET(PLATFORM_CLANG_PATH  "/riscv/tools_llvm/bin" CACHE INTERNAL "")
      if(TRUE)
        set(PLATFORM_CLANG_PATH
            "/riscv/llvm_21.1.8/bin"
            CACHE INTERNAL "")
        set(PLATFORM_CLANG_VERSION "-21")
      else()
        set(PLATFORM_CLANG_PATH
            "/riscv/21/bin"
            CACHE INTERNAL "")
        set(PLATFORM_CLANG_VERSION "-21")
      endif()
      if(USE_HW_FPU)
        # SET(PLATFORM_CLANG_SYSROOT "${PLATFORM_CLANG_PATH}/../lib/clang-runtimes/riscv32-none-eabi/riscv32_hard_fp/"
        # CACHE INTERNAL "")
        set(PLATFORM_CLANG_SYSROOT
            "${PLATFORM_CLANG_PATH}/../lib/clang-runtimes/riscv32-unknown-elf/riscv32_hard_fp/"
            CACHE INTERNAL "")
        set(PLATFORM_CLANG_C_FLAGS
            "--target=riscv32 -march=rv32imafc_zicsr -mabi=ilp32f -I${PLATFORM_CLANG_PATH}/../include "
            CACHE INTERNAL "")
      else()
        # SET(PLATFORM_CLANG_SYSROOT "${PLATFORM_CLANG_PATH}/../lib/clang-runtimes/riscv32-none-eabi/riscv32_soft_nofp/"
        # CACHE INTERNAL "")
        set(PLATFORM_CLANG_SYSROOT
            "${PLATFORM_CLANG_PATH}/../lib/clang-runtimes/riscv32-unknown-elf/riscv32_soft_nofp/"
            CACHE INTERNAL "")
        set(PLATFORM_CLANG_C_FLAGS
            "--target=riscv32 -march=rv32imac_zicsr -mabi=ilp32  -I${PLATFORM_CLANG_PATH}/../include"
            CACHE INTERNAL "")

      endif()
    endif()
  else() # ----------- GD32VF103
    set(PLATFORM_PREFIX riscv32-unknown-elf-)
    set(PLATFORM_C_FLAGS "-march=rv32imac -mabi=ilp32 ")
    if(WIN32)
      set(PLATFORM_TOOLCHAIN_PATH /c/gd32/toolchain/bin/) # Use /c/foo or c:\foo depending if you use mingw cmake or
                                                          # win32 cmake
    else()
      set(PLATFORM_TOOLCHAIN_PATH /opt/gd32/toolchain/bin/)
    endif()
  endif()
else()
  set(PLATFORM_PREFIX arm-none-eabi-)
  set(PLATFORM_C_FLAGS " ")
  if(WIN32)
    set(PLATFORM_TOOLCHAIN_PATH "/c/dev/arm83/bin")
  else()
    # /arm/arm_llvm_21.1.1/lib/clang-runtimes/arm-none-eabi SET(PLATFORM_CLANG_PATH /arm/arm_llvm_21.1.1/bin)
    # SET(PLATFORM_CLANG_VERSION "-21")
    set(PLATFORM_CLANG_PATH /arm/llvm_21.1/bin)
    set(PLATFORM_CLANG_VERSION "-21")
    # SET(PLATFORM_CLANG_PATH  /opt/llvm_arm/14/bin) SET(PLATFORM_CLANG_VERSION "")
    set(PLATFORM_TOOLCHAIN_PATH "/arm/14.2/bin/")
    set(PICO_SDK_PATH
        /opt/pico/pico-sdk
        CACHE INTERNAL "") # For PICO2040

  endif()
endif()
