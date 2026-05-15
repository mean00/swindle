add_library(swindleio_impl STATIC bmp_adc_ln.cpp bmp_rvTap_ln.cpp bmp_swdTap_ln.cpp bmp_tap_ln.cpp)
target_include_directories(swindleio_impl PRIVATE ..)
target_include_directories(swindleio_impl PRIVATE ../../private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)
