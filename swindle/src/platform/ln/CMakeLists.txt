add_library(swindleio_impl STATIC bmp_adc_ln.cpp bmp_rvTap_ln.cpp bmp_swdTap_ln.cpp bmp_tap_ln.cpp
                                  ../../template/swd_tap_stubs.cpp)
target_include_directories(swindleio_impl PRIVATE ../..)
target_include_directories(swindleio_impl PRIVATE ../../template)
target_include_directories(swindleio_impl PRIVATE ../../../private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)

if(USE_48PIN_PACKAGE)
  target_compile_definitions(swindleio_impl PUBLIC USE_48PIN_PACKAGE)
elseif(USE_64PIN_PACKAGE)
  target_compile_definitions(swindleio_impl PUBLIC USE_64PIN_PACKAGE)
else()
  message(FATAL_ERROR "You must select either USE_48PIN_PACKAGE or USE_64PIN_PACKAGE")
endif()
