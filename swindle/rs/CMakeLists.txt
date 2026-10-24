include(rnCmake)
# corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/lnbmp/Cargo.toml          CFLAGS ${LN_LTO_RUST_FLAGS}
# )
if(SWINDLE_HOSTED)
  message(FATAL_ERROR "This is not used !!")
  message(STATUS "Rust swindle in hosted mode")
  rust_add(rsbmp ${CMAKE_CURRENT_SOURCE_DIR}/rs_swindle_wrapper/Cargo.toml "" hosted)
else()
  message(STATUS "Rust swindle in target mode")
  set(LN_LTO_RUST_FLAGS ${LN_LTO_RUST_FLAGS} -C opt-level=s) # we try to make arm code smaller, we dont care for hosted
  # The SDI support is optional (SWINDLE_WITH_SDI, top level CMakeLists.txt): with
  # it off the Rust side of the seam is riscv_extra_sdi_stubs.rs instead of
  # riscv_extra_sdi.rs (rs_swindle/src/lib.rs), matching the C stubs.
  set(SWINDLE_RUST_SDI_FEATURE "")
  if(SWINDLE_WITH_SDI)
    set(SWINDLE_RUST_SDI_FEATURE sdi)
  endif()
  # The session arena (malloc_used.md) is optional the same way: with it off,
  # rs_swindle/src/lib.rs compiles session_arena_stubs.rs instead of
  # session_arena.rs, so the swindle_session_* symbols that
  # swindle/include/session_arena_shim.h redirects calloc/malloc/free to simply
  # forward to the heap.
  set(SWINDLE_RUST_ARENA_FEATURE "")
  if(SWINDLE_SESSION_ARENA)
    set(SWINDLE_RUST_ARENA_FEATURE session_arena)
  endif()
  # Platform backend for rust_esprit (esprit/rust/rp2xxx/README.md).  The C++
  # bridges for timer / multi-pulse / timing-ADC / I2C are not built on
  # RP2040/RP2350 (esprit/rust/CMakeLists.txt), so the Rust side must select
  # the `rp2040` backend instead of the `bluepill` (GD32/CH32) one.  Without
  # this, rust_esprit falls back to its default backend and the RP2040 link
  # fails with undefined `_rcuClockApb1` / `_rcuClockApb2` references.
  if(USE_RP2040 OR USE_RP2350)
    set(SWINDLE_RUST_PLATFORM_FEATURE rp2040)
  else()
    set(SWINDLE_RUST_PLATFORM_FEATURE bluepill)
  endif()
  # Package of the bluepill compatible chip (GD32/CH32 only) selects the pin mapping
  # in rs_pinout48_bluepill: USE_48PIN_PACKAGE -> bluepill48, USE_64PIN_PACKAGE -> bluepill64
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
  rust_add(rsbmp_wrapper ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml fake_std native ${SWINDLE_RUST_SDI_FEATURE} ${SWINDLE_RUST_ARENA_FEATURE} ${SWINDLE_RUST_PLATFORM_FEATURE} ${SWINDLE_RUST_PACKAGE_FEATURE} ${SWINDLE_RUST_LED_FEATURE} ${SWINDLE_RUST_LOGGER_FEATURE} ${SWINDLE_RUST_LOGGER_UART_FEATURE})
  # rs_rtos takes the kernel layout from esprit/rust/rs_rtos/src/kernel/esprit/generated.rs,
  # which the freertos_struct_header target regenerates from the kernel this build
  # links (esprit/cmake/lnFreeRTOS_structs.cmake).  Ordering, not a check: the file
  # has to be final before cargo reads the crate, otherwise a parallel build could
  # compile the previous layout and link a firmware whose static queues are too small.
  if(TARGET freertos_struct_header AND TARGET cargo-build_rsbmp_wrapper)
    add_dependencies(cargo-build_rsbmp_wrapper freertos_struct_header)
  endif()
  corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="native")
  if(SWINDLE_USE_NETWORK)
    corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="network")
  endif()
endif()
