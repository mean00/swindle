set(S
    ${BMP}/src
    CACHE INTERNAL "")
set(T
    ${BMP}/src/target
    CACHE INTERNAL "")
set(P
    ${BMP}/src/platforms
    CACHE INTERNAL "")
set(myB
    ${CMAKE_CURRENT_SOURCE_DIR}/
    CACHE INTERNAL "")
set(B
    ${myB}/src
    CACHE INTERNAL "")
set(BMP_EXTRA ${B}/../../blackmagic_addon/)
#
include(./boards.cmake)
generate_board_stub()
option(SWINDLE_HOSTED "Enable Hosted mode" OFF)
if(SWINDLE_HOSTED)
  include(./swindle_hosted.cmake)
else()
  include(./swindle_target.cmake)
endif()
