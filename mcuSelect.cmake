# We ony enable usb for arm and non small footprint
if(NOT DEFINED LN_ARCH)
  if(USE_CH32V3x) # RISCV
    if(LN_ENABLE_ETH)
      set(LN_ENABLED_BOARDS CH32F1 RP2040 RP2350 STM32F1 NRF51 NRF91 STM32F4 STM32G0 STM32H7 STM32H5 STM32L4 CACHE INTERNAL "")
    else()
      set(LN_ENABLED_BOARDS CH32F1 RP2040 RP2350 STM32F1 NRF51 NRF91 LPC11XX LPC15XX LPC17XX STM32F4 STM32G0 STM32H7 STM32H5 STM32L4 CACHE INTERNAL "")
    endif()
    # try without crystal... SET(LN_USE_INTERNAL_CLOCK True        CACHE INTERNAL "") SET(LN_MCU_SPEED          48000000
    # CACHE INTERNAL "") # 96 Mhz SET(LN_MCU_SPEED          96000000    CACHE INTERNAL "") # 96 Mhz SET(LN_MCU_SPEED
    # 96000000   CACHE INTERNAL "") # 144 Mhz
    set(LN_MCU_SPEED 144000000 CACHE INTERNAL "") # 144 Mhz
    set(LN_ARCH "RISCV" CACHE INTERNAL "")
    set(LN_MCU "CH32V3x" CACHE INTERNAL "")
    set(LN_MCU_RAM_SIZE 64 CACHE INTERNAL "")
    set(LN_MCU_FLASH_SIZE 256 CACHE INTERNAL "")
    if(LN_ENABLE_ETH)
      # +5 KiB vs before static kernel objects: idle stack (configMINIMAL_STACK_SIZE
      # = 512 words), timer task stack (configTIMER_TASK_STACK_DEPTH), both TCBs and
      # the timer command queue now live in .bss instead of the FreeRTOS heap.
      # +3 KiB more: entryTask and the usbd task carry their TCB + stack themselves
      # (lnCreateTaskStatic, malloc_used.md §8.1 S5).
      # +7 KiB more: gdbTask and usbserial too, right-sized to 4 + 2 KiB with
      # `mon stacks` (lnBmpTask.h).  The pool loses 12512 B of permanent
      # residents and 7168 B of its size, so ~5.3 KiB more is left for actual
      # allocation.
      # +2 KiB more: the logger / DMA / CDC / event-group / UART-timer statics
      # (malloc_used.md §8.1 B1, B2, B5, B6, S3b), 1632 B of .bss that used to
      # be ~1620 B of pool blocks - the same RAM, now out of reach of
      # fragmentation.
      set(LN_MCU_STATIC_RAM 59 CACHE INTERNAL "")
      set(LN_BOOTLOADER_SIZE 0 CACHE INTERNAL "")
    else()
      # 22 -> 25 KiB: entryTask + usbd are static tasks now (S5).
      # 25 -> 32 KiB: and so are gdbTask + usbserial, right-sized to 4 + 2 KiB
      # (S5, `mon stacks`).
      # 32 -> 34 KiB: the logger / CDC / event-group / UART-timer statics
      # (B1, B5, B6, S3b).
      # 34 -> 35 KiB: the Rust request-path buffers — register-file scratch,
      # FreeRTOS TCB cache and breakpoint tables are `.bss` now (C1, C2, C5, C6
      # in malloc_used.md §5), ~1.5 KiB that no longer allocates per packet.
      # §0.5 (C8) then added 504 B of FreeRTOS switch handlers; it fitted in the
      # 776 B of slack this value left, so the pool is unchanged (272 B left).
      # §0.6 (C10/C12) added 12 B more (the EventGroupRef tag on two groups);
      # still inside the slack, 260 B left.
      # 35 -> 36 KiB: §0.7 (C11) moved the three UART RX ring buffers (3 x 256 B)
      # out of the pool into the `RX_INSTANCES[]` static; 768 B of .bss did not
      # fit the 260 B of slack, so the pool gives up a whole KiB (28 -> 27 KiB).
      # §0.8 (C9) then spent 96 B of that slack on the RISC-V target context
      # (`CTX_SLOTS[2]`, 2 x 48 B): 456 B left, no budget change.
      set(LN_MCU_STATIC_RAM 36 CACHE INTERNAL "")
      set(LN_BOOTLOADER_SIZE 16 CACHE INTERNAL "") # 16
    endif()
    set(LN_SPEC "picolibc" CACHE INTERNAL "") # if not set we use nano
    set(LN_USB_NB_CDC 3 CACHE INTERNAL "") # 2 CDC interfaces
    set(LN_MCU_EEPROM_SIZE 1 CACHE INTERNAL "")
    if(SWINDLE_USE_CH32V208)
      set(LN_MCU_XTAL_CLOCK 25 CACHE INTERNAL "")
    endif()
  else()
    if(USE_GD32F3)
      set(LN_ENABLED_BOARDS CH32F1 RP2040 RP2350 STM32F1 NRF51 NRF91 LPC11XX LPC15XX LPC17XX STM32F4 STM32G0 STM32H7 STM32H5 STM32L4 CACHE INTERNAL "")
      set(LN_ARCH "ARM" CACHE INTERNAL "")
      set(LN_MCU "M4" CACHE INTERNAL "")
      set(LN_MCU_FLASH_SIZE 256 CACHE INTERNAL "")
      set(LN_MCU_RAM_SIZE 39 CACHE INTERNAL "")
      # 14 -> 17 KiB: the Idle task (stack + TCB), the timer service task (stack +
      # TCB) and the timer command queue are now static (.bss) instead of heap.
      # 17 -> 20 KiB: and so are entryTask and the usbd task (lnCreateTaskStatic,
      # malloc_used.md §8.1 S5).
      # 20 -> 27 KiB: gdbTask + usbserial too, right-sized to 4 + 2 KiB with
      # `mon stacks` (S5).  They were 12512 B of the pool; the 7 KiB the budget
      # gives up costs less than they did, so the pool gains ~5.3 KiB of usable
      # space and no fragmentation can reach a task stack any more.
      # 27 -> 29 KiB: the logger / DMA / CDC / event-group / UART-timer statics
      # (B1, B2, B5, B6, S3b).
      # 29 -> 30 KiB: the Rust request-path buffers — the register-file scratch
      # (320 B), the FreeRTOS TCB cache (heap Vec -> heapless, ~1 KiB) and the two
      # breakpoint tables (256 B) no longer allocate per GDB packet (C1, C2, C5,
      # C6 in malloc_used.md §5).  The pool drops to 8 KiB, which is still more
      # than the ~400 B of permanent residents left in it.
      # §0.5 (C8) then added 504 B of FreeRTOS switch handlers, which came out of
      # the 804 B of slack, not out of the pool (300 B left); §0.6 (C10/C12)
      # added another 8 B there (292 B left).
      # 30 -> 31 KiB: §0.7 (C11) moved the three UART RX ring buffers (3 x 256 B)
      # out of the pool into the `RX_INSTANCES[]` static.  768 B of .bss plus the
      # slack the 8 B above already ate did not fit the 30 KiB budget, so the
      # pool gives up a whole KiB (8 -> 7 KiB) instead of 768 B.
      # §0.8 (C9) then spent 96 B of that slack on the RISC-V target context
      # (`CTX_SLOTS[2]`, 2 x 48 B): 484 B left, no budget change.
      set(LN_MCU_STATIC_RAM 31 CACHE INTERNAL "")
      # SET(LN_MCU_SPEED 72000000    CACHE INTERNAL "") #=> ok
      set(LN_MCU_SPEED 96000000 CACHE INTERNAL "") # => ok
      set(LN_USB_NB_CDC 2 CACHE INTERNAL "")

      # SET(LN_MCU_SPEED  72000000    CACHE INTERNAL "") #=> ok
      set(LN_SPEC "picolibc" CACHE INTERNAL "") # if not set we use nano
      set(LN_BOOTLOADER_SIZE 8 CACHE INTERNAL "")
    elseif(USE_RP2040)
      set(LN_ENABLED_BOARDS CH32F1 RP2040 RP2350 STM32F1 NRF51 NRF91 LPC11XX LPC15XX LPC17XX
          STM32F4 STM32G0 STM32H7 STM32H5 STM32L4 CACHE INTERNAL "")
      set(LN_ARCH "ARM" CACHE INTERNAL "")
      set(LN_MCU "RP2040" CACHE INTERNAL "")
      set(LN_USB_NB_CDC 3 CACHE INTERNAL "") # 2 CDC interfaces
      if(USE_RP2040_PURE_RAM)
        set(LN_MCU_FLASH_SIZE 128 CACHE INTERNAL "")
        set(LN_MCU_RAM_SIZE 64 CACHE INTERNAL "")
      else()
        set(LN_MCU_RAM_SIZE 200 CACHE INTERNAL "")
        set(LN_MCU_FLASH_SIZE 2048 CACHE INTERNAL "")
      endif()
      set(LN_MCU_SPEED 125000000 CACHE INTERNAL "")
      # 11 -> 14 KiB: static kernel objects (idle task, timer task, timer queue; an
      # SMP build (ESPRIT_MULTICORE=ON) adds one passive idle task).
      # 14 -> 37 KiB: the four BMP tasks (entryTask, usbd, gdbTask/socketTask,
      # usbserial) keep their TCB + stack in .bss, ~23 KiB, which this part can
      # give away (lnBmpTask.h, malloc_used.md §8.1 S5).
      # 37 -> 31 KiB: gdbTask + usbserial are right-sized to 4 + 2 KiB (S5), so
      # their storage drops from 13024 B to 6880 B and the pool gets the 6 KiB.
      # 31 -> 33 KiB: the logger / CDC / event-group / UART-timer statics
      # (malloc_used.md §8.1 B1, B5, B6, S3b); the DMA mutexes of B2 are
      # bluepill-only, they are not built here.
      set(LN_MCU_STATIC_RAM 33 CACHE INTERNAL "")
      set(LN_MCU_EEPROM_SIZE 2 CACHE INTERNAL "")
      set(LN_BOOTLOADER_SIZE 0 CACHE INTERNAL "")
      set(LN_SPEC "picolibc" CACHE INTERNAL "") # if not set we use nano
    elseif(USE_RP2350)
      set(LN_ENABLED_BOARDS CH32F1 RP2040 RP2350 STM32F1 NRF51 NRF91 LPC11XX LPC15XX LPC17XX
          STM32F4 STM32G0 STM32H7 STM32H5 STM32L4 CACHE INTERNAL "")
      set(LN_ARCH "ARM" CACHE INTERNAL "")
      set(LN_MCU "RP2350" CACHE INTERNAL "")
      set(LN_USB_NB_CDC 3 CACHE INTERNAL "") # 2 CDC interfaces
      if(USE_RP2040_PURE_RAM)
        set(LN_MCU_FLASH_SIZE 300 CACHE INTERNAL "")
        set(LN_MCU_RAM_SIZE 200 CACHE INTERNAL "")
      else()
        set(LN_MCU_RAM_SIZE 500 CACHE INTERNAL "")
        set(LN_MCU_FLASH_SIZE 4096 CACHE INTERNAL "")
      endif()
      set(LN_MCU_SPEED 150000000 CACHE INTERNAL "")
      # 11 -> 14 KiB: static kernel objects (idle task(s), timer task, timer queue).
      # 14 -> 37 KiB: the four BMP tasks keep their TCB + stack in .bss (S5).
      # 37 -> 31 KiB: gdbTask + usbserial right-sized to 4 + 2 KiB (S5).
      # 31 -> 33 KiB: the logger / DMA / CDC / event-group / timer statics
      # (malloc_used.md §8.1 B1, B2, B5, B6, S3b) moved out of the pool.
      set(LN_MCU_STATIC_RAM 33 CACHE INTERNAL "")
      set(LN_MCU_EEPROM_SIZE 2 CACHE INTERNAL "")
      set(LN_BOOTLOADER_SIZE 0 CACHE INTERNAL "")
      set(LN_SPEC "picolibc" CACHE INTERNAL "") # if not set we use nano

    else() # Small bluepill style
      set(LN_ARCH "ARM" CACHE INTERNAL "")
      set(LN_MCU "M3" CACHE INTERNAL "")
      set(LN_MCU_RAM_SIZE 20 CACHE INTERNAL "")
      set(LN_MCU_SPEED 72000000 CACHE INTERNAL "")
      set(LN_USB_NB_CDC 2 CACHE INTERNAL "") # 2 CDC interfaces
      set(LN_ENABLED_BOARDS STM32F1 CACHE INTERNAL "")
      # 11 -> 14 KiB: static kernel objects (idle task, timer task, timer queue).
      # 14 -> 17 KiB: entryTask + usbd are static tasks too (S5).  There is no room
      # for the two 6 KiB tasks on a 20 KiB part, so they stay on the heap.
      # Not built by any preset: adding the B1/B2/B5/B6/S3b statics (1632 B) here
      # would need a 19 KiB budget and leave no pool, and the heap is 2 KiB.
      set(LN_MCU_STATIC_RAM 17 CACHE INTERNAL "")
      set(LN_MCU_FLASH_SIZE 128 CACHE INTERNAL "")
      set(LN_MCU_EEPROM_SIZE 0 CACHE INTERNAL "")
      set(LN_BOOTLOADER_SIZE 0 CACHE INTERNAL "")
      set(LN_SPEC "picolibc" CACHE INTERNAL "") # if not set we use nano
    endif() # GD32F3
  endif() # CH32V3
endif(NOT DEFINED LN_ARCH)
message(STATUS "Architecture ${LN_ARCH}, MCU=${LN_MCU}")
