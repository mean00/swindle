add_library(swindleio_impl STATIC bmp_adc_esp.cpp bmp_rvTap_esp_spi.cpp bmp_swdTap_esp_spi.cpp bmp_tap_esp_spi.cpp
                                  ../../template/swd_tap_stubs.cpp)
target_include_directories(swindleio_impl PRIVATE ../..)
target_include_directories(swindleio_impl PRIVATE ../../template)
target_include_directories(swindleio_impl PRIVATE ../../../private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)
