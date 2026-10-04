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
  rust_add(rsbmp_wrapper ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml fake_std native ${SWINDLE_RUST_SDI_FEATURE} ${SWINDLE_RUST_PLATFORM_FEATURE})
  corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="native")
  if(SWINDLE_USE_NETWORK)
    corrosion_add_target_rustflags(rsbmp_wrapper --cfg feature="network")
  endif()
endif()
