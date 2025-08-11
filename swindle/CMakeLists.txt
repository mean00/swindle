SET(S ${BMP}/src CACHE INTERNAL "")
SET(T ${BMP}/src/target CACHE INTERNAL "")
SET(P ${BMP}/src/platforms CACHE INTERNAL "")
SET(myB ${CMAKE_CURRENT_SOURCE_DIR}/ CACHE INTERNAL "")
SET(B ${myB}/src CACHE INTERNAL "")
SET(BMP_EXTRA  ${B}/../../blackmagic_addon/)
#
if(USE_INVERTED_NRST )
  ADD_DEFINITIONS("-DSWINDLE_INVERTED_RESET=1")
endif()
#
include(./boards.cmake)
GENERATE_BOARD_STUB()
OPTION(SWINDLE_HOSTED  "Enable Hosted mode" OFF)
IF (SWINDLE_HOSTED)
  include(./swindle_hosted.cmake)
ELSE()
  include(./swindle_target.cmake)
ENDIF()


