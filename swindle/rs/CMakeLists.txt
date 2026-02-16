include(rnCmake)
#corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/lnbmp/Cargo.toml          CFLAGS ${LN_LTO_RUST_FLAGS} )
IF(SWINDLE_HOSTED)
  MESSAGE(FATAL_ERROR  "This is not used !!")
  MESSAGE(STATUS "Rust swindle in hosted mode")
  RUST_ADD( rsbmp ${CMAKE_CURRENT_SOURCE_DIR}/rs_swindle_wrapper/Cargo.toml "" hosted)
ELSE()
  MESSAGE(STATUS "Rust swindle in target mode")
  SET(LN_LTO_RUST_FLAGS ${LN_LTO_RUST_FLAGS} -C opt-level=s)  # we try to make arm code smaller, we dont care for hosted
  RUST_ADD( rsbmp_wrapper ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml )
  corrosion_add_target_rustflags( rsbmp_wrapper --cfg feature="native")
ENDIF()




