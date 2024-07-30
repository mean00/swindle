include(rnCmake)
#corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/lnbmp/Cargo.toml          CFLAGS ${LN_LTO_RUST_FLAGS} )

IF (LN_BMP_HOSTED)
    RUST_ADD( rsbmp ${CMAKE_CURRENT_SOURCE_DIR}/rs_swindle/Cargo.toml hosted)
ELSE()
    SET(LN_LTO_RUST_FLAGS ${LN_LTO_RUST_FLAGS} -C opt-level=s)  # we try to make arm code smaller, we dont care for hosted
    RUST_ADD( rsbmp ${CMAKE_CURRENT_SOURCE_DIR}/rs_swindle/Cargo.toml )
ENDIF()




