include(rnCmake)
# corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/lnbmp/Cargo.toml          CFLAGS ${LN_LTO_RUST_FLAGS}
# )
if(SWINDLE_HOSTED)
  message(FATAL_ERROR "This is not used !!")
  message(STATUS "Rust swindle in hosted mode")
  rust_add(rsbmp ${CMAKE_CURRENT_SOURCE_DIR}/rs_swindle_wrapper/Cargo.toml "" hosted)
else()
  message(STATUS "Rust swindle in target mode")
  set(LN_LTO_RUST_FLAGS ${LN_LTO_RUST_FLAGS} -C opt-level=z -Z unstable-options -C panic=immediate-abort) # we try to make arm code smaller, we dont care for hosted
  set(LN_RUST_BUILD_FLAGS -Z build-std=core,alloc)
  # The SDI support is optional (SWINDLE_WITH_SDI, top level CMakeLists.txt): with
  # it off the Rust side of the seam is riscv_extra_sdi_stubs.rs instead of
  # riscv_extra_sdi.rs (rs_swindle/src/lib.rs), matching the C stubs.
  set(SWINDLE_RUST_SDI_FEATURE "")
  if(SWINDLE_WITH_SDI)
    set(SWINDLE_RUST_SDI_FEATURE sdi)
  endif()
  # Pure Rust ARM drivers (rs_stm32f1f4 / rs_swindle_stm32g0 / rs_swindle_rp2040 / rs_swindle_rp2350_arm): enabled by
  # swindle_common.cmake when the C source of a ported family is built
  # (SWINDLE_RUST_ARM_TARGETS). Each family name maps to its cargo feature; the probe chain
  # (rs_swindle_wrapper/src/arm_probe_chain.rs) dispatches to the Rust driver of the family when
  # its feature is on and to the C probe otherwise. `rust_rp2350` covers both RP2350 drivers,
  # the Arm one used from that chain and the RISC-V one from src/riscv_shim.rs.
  set(SWINDLE_RUST_ARM_FEATURE "")
  foreach(family ${SWINDLE_RUST_ARM_TARGETS})
    list(APPEND SWINDLE_RUST_ARM_FEATURE rust_${family})
  endforeach()
  # The session arena (malloc_used.md) is optional the same way: with it off,
  # rs_swindle/src/lib.rs compiles session_arena_stubs.rs instead of
  # session_arena.rs, so the swindle_session_* symbols that
  # swindle/include/session_arena_shim.h redirects calloc/malloc/free to simply
  # forward to the heap.
  set(SWINDLE_RUST_ARENA_FEATURE "")
  if(SWINDLE_SESSION_ARENA)
    set(SWINDLE_RUST_ARENA_FEATURE session_arena)
  endif()
  # Platform backend for rust_esprit (esprit/rust/rs_mcus/rp2xxx/README.md).  The C++
  # bridges for timer / multi-pulse / timing-ADC / I2C are not built on
  # RP2040/RP2350 (esprit/rust/CMakeLists.txt), so the Rust side must select
  # the `rp2040` backend instead of the `bluepill` (GD32/CH32) one.  Without
  # this, rust_esprit falls back to its default backend and the RP2040 link
  # fails with undefined `_rcuClockApb1` / `_rcuClockApb2` references.
  # The chip, not just the family, is part of the feature: `rp2350` also picks the
  # RP2350 address map (IO_BANK0 / PADS_BANK0, SPI, RESETS, CLOCKS) for the
  # rs_*_rp2xxx backends, which `rp2040` alone would place at the RP2040 addresses,
  # and `ch32v3x` picks the CH32V3 side of the bluepill map (addresses + AHBENR
  # bits) instead of the ARM one.
  if(USE_RP2350)
    set(SWINDLE_RUST_PLATFORM_FEATURE rp2350)
  elseif(USE_RP2040)
    set(SWINDLE_RUST_PLATFORM_FEATURE rp2040)
  else()
    set(SWINDLE_RUST_PLATFORM_FEATURE bluepill)
  endif()
  # Chip-level selector on top of the family one (empty when the family has a
  # single chip view, which is the case for the ARM bluepill parts).
  if(USE_CH32V3x)
    set(SWINDLE_RUST_CHIP_FEATURE ch32v3x)
  elseif(NOT (USE_RP2040 OR USE_RP2350))
    set(SWINDLE_RUST_CHIP_FEATURE gd32fx)
  else()
    set(SWINDLE_RUST_CHIP_FEATURE "")
  endif()
  # Package of the bluepill compatible chip (GD32/CH32 only) selects the pin mapping
  # in rs_pinout48_bluepill: USE_48PIN_PACKAGE -> bluepill48, USE_64PIN_PACKAGE -> bluepill64
  # The same two features also enable the probe board table (rs_swindle_board, board
  # file chosen by swindle/board.cmake) in rs_swindle and rs_swindle_leds.
  set(SWINDLE_RUST_PACKAGE_FEATURE "")
  if(NOT (USE_RP2040 OR USE_RP2350))
    if(USE_48PIN_PACKAGE)
      set(SWINDLE_RUST_PACKAGE_FEATURE bluepill48)
    elseif(USE_64PIN_PACKAGE)
      set(SWINDLE_RUST_PACKAGE_FEATURE bluepill64)
    else()
      message(FATAL_ERROR "Bluepill target: USE_48PIN_PACKAGE or USE_64PIN_PACKAGE must be set")
    endif()
  endif()
  # LED variant (src/swindle_gd32*.cpp -> rs_swindle_leds): one WS2812B led when USE_WS2812,
  # plain open drain leds otherwise. Only relevant for bluepill compatible targets (GD32/CH32).
  set(SWINDLE_RUST_LED_FEATURE "")
  if(USE_WS2812 AND NOT (USE_RP2040 OR USE_RP2350))
    set(SWINDLE_RUST_LED_FEATURE ws2812)
  endif()
  # RP2040 / RP2350 board table (swindle/rs/rs_swindle_board/src/rp2xxx/).
  # Empty on the other platforms.
  set(SWINDLE_RUST_RP_BOARD_FEATURE "")
  if(USE_RP2040 OR USE_RP2350)
    set(SWINDLE_RUST_RP_BOARD_FEATURE rp_carrier)
    if(USE_RP_ZERO)
      list(APPEND SWINDLE_RUST_RP_BOARD_FEATURE rp_zero)
    endif()
  endif()
  # Inverted target nRST (USE_INVERTED_NRST, a MOSFET on the board): reset_pin.rs then
  # drives the line push-pull / active high instead of open drain / active low. It is
  # the Rust counterpart of the bmp_reset.cpp / bmp_reset_inv.cpp selection in
  # swindle_target.cmake, so the two halves must always agree on the polarity.
  set(SWINDLE_RUST_NRST_FEATURE "")
  if(USE_INVERTED_NRST)
    set(SWINDLE_RUST_NRST_FEATURE nrst_inverted)
  endif()
  # Esprit logger in Rust (esprit/CMakeLists.txt: USE_RUST_LOGGER) and the UART it
  # writes to (LN_DEBUG_UART, instance 0 when unset) — rust_esprit/src/logger_c.rs.
  set(SWINDLE_RUST_LOGGER_FEATURE "")
  set(SWINDLE_RUST_LOGGER_UART_FEATURE "")
  if(USE_RUST_LOGGER)
    set(SWINDLE_RUST_LOGGER_FEATURE rust_logger)
    if(LN_DEBUG_UART)
      set(SWINDLE_RUST_LOGGER_UART_FEATURE logger_uart${LN_DEBUG_UART})
    endif()
    message(STATUS "Rust logger: UART instance ${LN_DEBUG_UART}")
  endif()
  set(SWINDLE_RUST_NET_FEATURE "")
  if(SWINDLE_USE_NETWORK)
    set(SWINDLE_RUST_NET_FEATURE network)
  endif()
  # usb.md P2: the Rust device stack above tinyUSB (rs_tinyusb::stack) replaces the C++
  # lnUsbStack; LN_RUST_USB is set at the top level for the families that use it.
  set(SWINDLE_RUST_USB_FEATURE "")
  set(SWINDLE_RUST_CDC_FEATURE "")
  if(LN_RUST_USB)
    set(SWINDLE_RUST_USB_FEATURE usb_stack)
    # How many CDC-ACM interfaces - rs_usb checks it against the MCU's endpoint budget
    # at compile time (usb.md P4). One feature, so the count cannot drift from CMake.
    if("${LN_USB_NB_CDC}" STREQUAL "3")
      set(SWINDLE_RUST_CDC_FEATURE cdc3)
    elseif("${LN_USB_NB_CDC}" STREQUAL "2")
      set(SWINDLE_RUST_CDC_FEATURE cdc2)
    endif()
  endif()
  rust_add(rsbmp_wrapper ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml fake_std native ${SWINDLE_RUST_USB_FEATURE} ${SWINDLE_RUST_CDC_FEATURE} ${SWINDLE_RUST_SDI_FEATURE} ${SWINDLE_RUST_ARM_FEATURE} ${SWINDLE_RUST_ARENA_FEATURE} ${SWINDLE_RUST_PLATFORM_FEATURE} ${SWINDLE_RUST_CHIP_FEATURE} ${SWINDLE_RUST_PACKAGE_FEATURE} ${SWINDLE_RUST_RP_BOARD_FEATURE} ${SWINDLE_RUST_LED_FEATURE} ${SWINDLE_RUST_NRST_FEATURE} ${SWINDLE_RUST_LOGGER_FEATURE} ${SWINDLE_RUST_LOGGER_UART_FEATURE} ${SWINDLE_RUST_NET_FEATURE})
  # rs_rtos takes the kernel layout from esprit/rust/rs_rtos/src/kernel/esprit/generated.rs,
  # which the freertos_struct_header target regenerates from the kernel this build
  # links (esprit/cmake/lnFreeRTOS_structs.cmake).  Ordering, not a check: the file
  # has to be final before cargo reads the crate, otherwise a parallel build could
  # compile the previous layout and link a firmware whose static queues are too small.
  if(TARGET freertos_struct_header AND TARGET cargo-build_rsbmp_wrapper)
    add_dependencies(cargo-build_rsbmp_wrapper freertos_struct_header)
  endif()
  corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="native")
  if(LN_MCU_SPEED)
    corrosion_set_env_vars(rsbmp_wrapper "LN_MCU_SPEED=${LN_MCU_SPEED}")
  endif()
  if(LN_XTAL_VALUE)
    corrosion_set_env_vars(rsbmp_wrapper "LN_XTAL_VALUE=${LN_XTAL_VALUE}")
  endif()
  if(DEFINED STR_BOARD_ENABLED)
    corrosion_set_env_vars(rsbmp_wrapper "LN_BOARDS_ENABLED=${STR_BOARD_ENABLED}")
  endif()
  if(SWINDLE_USE_NETWORK)
    corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="network")
  endif()
endif()
