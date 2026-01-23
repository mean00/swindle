add_library(swindleio_impl STATIC bmp_adc_ln.cpp bmp_rvTap_ln.cpp bmp_swdTap_ln.cpp bmp_tap_ln.cpp
../swd_tap_stubs.cpp
)
target_include_directories(swindleio_impl PRIVATE ..)
target_include_directories(swindleio_impl PRIVATE ../../private_include)
TARGET_LINK_LIBRARIES( swindleio_impl PRIVATE esprit_dev )


IF( USE_48PIN_PACKAGE)
  target_compile_definitions( swindleio_impl PUBLIC USE_48PIN_PACKAGE)
ELSEIF( USE_64PIN_PACKAGE)
  target_compile_definitions( swindleio_impl PUBLIC USE_64PIN_PACKAGE)
ELSE()
  MESSAGE(FATAL_ERROR "You must select either USE_48PIN_PACKAGE or USE_64PIN_PACKAGE")
ENDIF()

