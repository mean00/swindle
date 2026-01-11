#IF(TRUE)
RP_PIO_GENERATE( ${CMAKE_CURRENT_SOURCE_DIR}/swd.pio ${CMAKE_BINARY_DIR}/bmp_pio_swd.h)
RP_PIO_GENERATE( ${CMAKE_CURRENT_SOURCE_DIR}/rvswd.pio ${CMAKE_BINARY_DIR}/bmp_pio_rvswd.h)
SET( EXTRA_SOURCE ${CMAKE_BINARY_DIR}/bmp_pio_swd.h ${CMAKE_BINARY_DIR}/bmp_pio_rvswd.h) # ${B}/bmp_rvTap${EXTRA}.cpp   ${B}/bmp_swdTap${EXTRA}.cpp ${B}/bmp_tap_rp2040.cpp  )
#ELSE()
#SET( EXTRA_SOURCE  ${B}/bmp_rvTap.cpp   ${B}/bmp_swdTap.cpp  ${B}/bmp_tap_gpio.cpp )
#ENDIF()
IF(USE_RP_CARRIER)
  MESSAGE(STATUS "RP2040 using carrier board layout")
  ADD_DEFINITIONS("-DUSE_RP_CARRIER")
ENDIF()


add_library(swindleio_impl STATIC bmp_adc_rp2040.cpp bmp_rvTap_rp2040.cpp
bmp_swdTap_rp2040.cpp bmp_tap_rp2040.cpp ${EXTRA_SOURCE}
../swd_tap_stubs.cpp
)
target_include_directories(swindleio_impl PRIVATE ..)
target_include_directories(swindleio_impl PRIVATE ../../private_include)
TARGET_LINK_LIBRARIES( swindleio_impl PRIVATE esprit_dev )

