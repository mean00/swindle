# ===========================================================================
# Probe board: which wiring the firmware drives, and the C view of it.
#
# The pinout, ADC sense input, UART and USB assignation and the LEDs of every
# board are described in Rust, one file per board, in
#
#     swindle/rs/rs_swindle_board/src/<board>.rs
#
# This file picks one of them from the CMake options that already select the
# target (it is included by swindle_common.cmake, before the platform and Rust
# subdirectories, so both the C side and cargo see the choice) and renders it as
#
#     ${CMAKE_BINARY_DIR}/generated/bmp_pinout_board.h
#
# with swindle/tools/gen_board_header. That keeps C on compile-time constants
# (`_mapping[]`, PIN_ADC_NRESET_*, LN_UART_*, LN_*_INSTANCE) without a second copy
# of the numbers: bmp_pinout.h includes the generated header.
#
# Results:
#   SWINDLE_BOARD           board file name; the matching cargo feature of
#                           rs_swindle_board (ln48 / ln64 today) is enabled through
#                           the bluepill48 / bluepill64 features of rs_swindle and
#                           rs_swindle_leds (swindle/rs/CMakeLists.txt)
#   SWINDLE_BOARD_FILE      the Rust file the header is generated from
#   swindle_board_header    custom target producing bmp_pinout_board.h; every
#                           target compiling swindle/src must depend on it
# ===========================================================================
set(SWINDLE_BOARD "")
set(SWINDLE_BOARD_FILE "")

if(NOT SWINDLE_HOSTED)
  if(USE_RP2040 OR USE_RP2350)
    # Not migrated to the board table yet: these still take their pinout from
    # src/platform/rp2040/bmp_pinout_rp2040*.h.
    message(STATUS "Probe board: RP2040/RP2350 (C pinout, board table not used yet)")
  elseif("${LN_MCU}" STREQUAL "ESP32")
    # Same: src/platform/esp32_pinout/{mini,wroom}/bmp_pinout.h.
    message(STATUS "Probe board: ESP32 (C pinout, board table not used yet)")
  elseif(USE_48PIN_PACKAGE)
    set(SWINDLE_BOARD ln48)
    set(SWINDLE_BOARD_FILE ${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle_board/src/bluepill/ln_48pins.rs)
  elseif(USE_64PIN_PACKAGE)
    set(SWINDLE_BOARD ln64)
    set(SWINDLE_BOARD_FILE ${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle_board/src/bluepill/ln_64pins.rs)
  else()
    message(FATAL_ERROR "You must select either USE_48PIN_PACKAGE or USE_64PIN_PACKAGE")
  endif()
  # The pin encoding the board file is written with (see bluepill/pins.rs): the
  # generator includes it as `crate::pins`, the crate re-exports the same family
  # module there.
  set(SWINDLE_BOARD_PINS ${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle_board/src/bluepill/pins.rs)
endif()

if(SWINDLE_BOARD)
  set(SWINDLE_BOARD_DESC ${CMAKE_CURRENT_SOURCE_DIR}/rs/rs_swindle_board/src/board_desc.rs)
  set(SWINDLE_BOARD_TOOL ${CMAKE_CURRENT_SOURCE_DIR}/tools/gen_board_header/src/main.rs)
  set(SWINDLE_BOARD_HEADER ${CMAKE_BINARY_DIR}/generated/bmp_pinout_board.h)
  # The generator is a host binary: it is built by the same rustc cargo uses, not
  # by the target toolchain, because it only formats the table.
  #
  # Corrosion only publishes _CORROSION_RUSTC while *its* directory is processed,
  # which happens after this file (swindle_common.cmake includes this before
  # add_subdirectory(rs)), so on a fresh build directory nothing is set yet. The
  # fallback therefore has to unset the variable first: find_program() does not
  # search when its result variable is already defined — an empty value makes it
  # silently return empty, and the generator command below would then read
  # `cmake -E env … --edition 2024`, i.e. fail with "unknown option '--edition'".
  set(_swindle_board_rustc "${_CORROSION_RUSTC}")
  if(NOT _swindle_board_rustc)
    set(_swindle_board_rustc "${Rust_COMPILER}")
  endif()
  if(NOT _swindle_board_rustc)
    unset(_swindle_board_rustc)
    find_program(_swindle_board_rustc NAMES rustc REQUIRED)
  endif()
  if(NOT _swindle_board_rustc)
    message(FATAL_ERROR "board.cmake: no rustc found to build the board header generator")
  endif()
  message(STATUS "Board header generator: rustc ${_swindle_board_rustc}")
  # The chosen board is part of the generator's input, so switching package (or
  # platform) in an existing build directory has to re-render the header. A stamp
  # file carries that, and is only rewritten when the choice actually changes.
  set(_swindle_board_stamp ${CMAKE_BINARY_DIR}/generated/board_selection.txt)
  set(_swindle_board_choice "${SWINDLE_BOARD} (${SWINDLE_BOARD_FILE})")
  set(_swindle_board_previous "")
  if(EXISTS ${_swindle_board_stamp})
    file(READ ${_swindle_board_stamp} _swindle_board_previous)
  endif()
  if(NOT _swindle_board_previous STREQUAL _swindle_board_choice)
    file(WRITE ${_swindle_board_stamp} "${_swindle_board_choice}")
  endif()
  add_custom_command(
    OUTPUT ${SWINDLE_BOARD_HEADER}
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_BINARY_DIR}/generated
    # SWINDLE_BOARD_DESC / SWINDLE_BOARD_RS are the `include!` paths of the tool,
    # so they are read while compiling it, not while running it.
    COMMAND ${CMAKE_COMMAND} -E env "SWINDLE_BOARD_DESC_RS=${SWINDLE_BOARD_DESC}"
            "SWINDLE_BOARD_PINS_RS=${SWINDLE_BOARD_PINS}"
            "SWINDLE_BOARD_RS=${SWINDLE_BOARD_FILE}" ${_swindle_board_rustc} --edition 2024 -O
            -o ${CMAKE_BINARY_DIR}/gen_board_header${CMAKE_EXECUTABLE_SUFFIX} ${SWINDLE_BOARD_TOOL}
    COMMAND ${CMAKE_BINARY_DIR}/gen_board_header${CMAKE_EXECUTABLE_SUFFIX} -o ${SWINDLE_BOARD_HEADER}
    DEPENDS ${SWINDLE_BOARD_TOOL} ${SWINDLE_BOARD_DESC} ${SWINDLE_BOARD_PINS} ${SWINDLE_BOARD_FILE} ${_swindle_board_stamp}
    COMMENT "Rendering bmp_pinout_board.h from board ${SWINDLE_BOARD}"
    VERBATIM)
  add_custom_target(swindle_board_header DEPENDS ${SWINDLE_BOARD_HEADER})
  message(STATUS "Probe board: ${SWINDLE_BOARD} (pins from ${SWINDLE_BOARD_FILE})")
endif()
