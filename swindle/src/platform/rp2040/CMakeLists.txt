# IF(TRUE)
rp_pio_generate(${CMAKE_CURRENT_SOURCE_DIR}/swd.pio ${CMAKE_BINARY_DIR}/bmp_pio_swd.h)
rp_pio_generate(${CMAKE_CURRENT_SOURCE_DIR}/rvswd.pio ${CMAKE_BINARY_DIR}/bmp_pio_rvswd.h)
set(EXTRA_SOURCE ${CMAKE_BINARY_DIR}/bmp_pio_swd.h ${CMAKE_BINARY_DIR}/bmp_pio_rvswd.h) # ${B}/bmp_rvTap${EXTRA}.cpp
                                                                                        # ${B}/bmp_swdTap${EXTRA}.cpp
                                                                                        # ${B}/bmp_tap_rp2040.cpp  )

add_library(swindleio_impl STATIC bmp_adc_rp2040.cpp bmp_rvTap_rp2040.cpp bmp_swdTap_rp2040.cpp bmp_tap_rp2040.cpp
                                  ${EXTRA_SOURCE} ../../template/swd_tap_stubs.cpp)
# Board selection -> ln_bsp interface (linked PUBLIC into libswindle by
# swindle_target.cmake and PRIVATE into swindleio_impl here).
if(USE_RP_CARRIER)
  message(STATUS "RP2040 using carrier board layout")
  target_compile_definitions(ln_bsp INTERFACE USE_RP_CARRIER LN_UART_RX=GPIO9 LN_UART_TX=GPIO4)
else()
  target_compile_definitions(ln_bsp INTERFACE LN_UART_RX=GPIO5 LN_UART_TX=GPIO4)
endif()

target_include_directories(swindleio_impl PRIVATE ../..)
target_include_directories(swindleio_impl PRIVATE ../../template)
target_include_directories(swindleio_impl PRIVATE ../../../private_include)
target_link_libraries(swindleio_impl PRIVATE esprit_dev)
ln_setup_platform_impl(swindleio_impl)
