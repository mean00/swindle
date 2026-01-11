add_library(swindleio_impl STATIC bmp_adc_esp.cpp bmp_rvTap_esp.cpp bmp_swdTap_esp.cpp bmp_tap_esp.cpp
../swd_tap_stubs.cpp)
target_include_directories(swindleio_impl PRIVATE ..)
target_include_directories(swindleio_impl PRIVATE ../../private_include)
TARGET_LINK_LIBRARIES( swindleio_impl PRIVATE esprit_dev )
