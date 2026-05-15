add_library(swindleio_impl STATIC bmp_rvTap_fesp.cpp bmp_swdTap_fesp.cpp bmp_tap_fesp.cpp
                                  ../esp32_pinout/src/bmp_adc_esp.cpp)
target_include_directories(swindleio_impl PRIVATE ../..)
target_include_directories(swindleio_impl PRIVATE ../../template)
target_include_directories(swindleio_impl PRIVATE ../../../private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)
