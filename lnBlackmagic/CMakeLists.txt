

SET(S ${BMP}/src)
SET(T ${BMP}/src/target )
SET(P ${BMP}/src/platforms )

SET(myB ${CMAKE_CURRENT_SOURCE_DIR}/ )

SET(B ${myB}/src )

OPTION(LN_BMP_HOSTED  "Enable Hosted mode" OFF)


# Here are the boards that are removed, dont forget to edit bmp_disabledBoard if you change that list
# ${T}/lpc43xx.c   ${T}/nrf51.c ${T}/rp.c ${T}/sam4l.c ${T}/samx5x.c  ${T}/kinetis.c${T}/samd.c
#  ${T}/efm32.c   ${T}/msp432.c ${T}/stm32h7.c   ${T}/lpc546xx.c ${T}/lpc17xx.c     ${T}/lpc15xx.c  
# ${T}/nxpke04.c ${T}/sam3x.c
# dont need cortexa ${T}/cortexa.c
# The one i use...
SET(BOARDS      ${T}/lpc_common.c       ${T}/stm32f1.c ${T}/stm32g0.c   ${T}/ch32f1.c ${T}/lpc11xx.c     ${B}/bmp_disabledBoard.cpp
                ${T}/stm32l0.c          ${T}/stm32f4.c  ${T}/stm32l4.c       ${T}/rp.c
        )

include_directories(${CMAKE_SOURCE_DIR}/blackmagic/libopencm3/include)

IF (LN_BMP_HOSTED) # Hosted mode
    MESSAGE(STATUS "Building for hosted mode (${LN_BMP_HOSTED})")
    ADD_DEFINITIONS("-DPC_HOSTED=1")
    ADD_DEFINITIONS("-DHOSTED_BMP_ONLY=1")
    ADD_DEFINITIONS("-DENABLE_DEBUG=1")

    # Set a dummy configuration for lnArduino so it builds
    SET(LN_EXT "arm_gd32fx")
    SET(LN_MCU "M3")
    ADD_DEFINITIONS("-DLN_ARCH=LN_ARCH_ARM")
    include(${ARDUINO_GD32_FREERTOS}/setup.cmake)

    include_directories(${S})
    include_directories(../)
    include_directories(../lnArduino/legacy/boards/bluepill)

    SET(HOSTED ${BMP}/src/platforms/hosted )
    SET(PC ${BMP}/src/platforms/hosted )

    include_directories( ${HOSTED})
    SET(BM_HOSTED
            ${HOSTED}/bmp_remote.c ${HOSTED}/bmp_serial.c      ${HOSTED}/dap.c
            ${HOSTED}/platform.c   ${HOSTED}/remote_jtagtap.c  ${HOSTED}/remote_swdptap.c
            #${PC}/cl_utils.c   
            #${PC}/gdb_if.c    
            ${PC}/serial_unix.c  ${PC}/utils.c
            ${PC}/cli.c       ${PC}/probe_info.c
            ${T}/jtag_scan.c  ${T}/jtag_devs.c    ${T}/adiv5_jtagdp.c
            ${B}/stubs.cpp
            )
ELSE()  # "Normal" /Standalone mode
  MESSAGE(STATUS "Building for embedded mode (${LN_BMP_HOSTED})")
  ADD_DEFINITIONS("-DPLATFORM_IDENT=\"lnBMP\"")
  ADD_DEFINITIONS("-DPC_HOSTED=0")
  include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)
  #  add lnBlackmagic target code and host connection stuff
  SET(BRIDGE_SRCS ${B}/bridge.cpp ${B}/bmp_gpio.cpp  ${B}/bmp_stubs.cpp
          ${B}/bmp_swdTap.cpp   ${B}/bmp_serial.cpp  
          ${B}/bmp_rs_gdb.cpp
          #${B}/bmp_gdb.cpp
          ${B}/bmp_jtagstubs.cpp  ${B}/stubs.cpp
          )
  include_directories( ${CMAKE_SOURCE_DIR}/lnBlackmagic/include )
ENDIF()
SET(BM_SRC          ${S}/command.c  ${S}/crc32.c        ${S}/exception.c    #${S}/gdb_main.c ${S}/gdb_packet.c
                    ${S}/hex_utils.c ${S}/morse.c ${S}/remote.c ${S}/timing.c)
SET(BM_TARGET       ${T}/adiv5.c    ${T}/adiv5_swdp.c   ${T}/cortexm.c    ${T}/target_flash.c ${T}/gdb_reg.c
                    ${T}/lmi.c      
                    ${T}/target.c ${T}/sfdp.c
                        # ${S}/gdb_hostio.c 
                        ) #${T}/jtag_scan.c) ${T}/jtag_devs.c    ${T}/adiv5_jtagdp.c  

SET(BM_PLATFORM_COMMON   ${B}/bmp_interface.cpp ${B}/bmp_string.cpp ${B}/bmp_utils.cpp 
                         ${B}/bmp_gdb_cmd.cpp ${B}/bmp_thread_cache.cpp
                         #${BMP}/../rs/lnbmp/c_interface/bmp_interface_c.cpp
                        )

add_library(lnBlackmagic STATIC ${BM_SRC} ${BRIDGE_SRCS} ${BM_TARGET} ${BM_HOSTED} ${BM_PLATFORM_COMMON} ${BOARDS})
IF(USE_STLINK_PINOUT)
        target_compile_definitions(lnBlackmagic PRIVATE USE_STLINK_PINOUT)
ENDIF(USE_STLINK_PINOUT)
target_include_directories(lnBlackmagic PRIVATE  ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config )
target_include_directories(lnBlackmagic PRIVATE  ${myB}/private_include)

include_directories( ${BMP}/src/include)
include_directories( ${BMP}/src)
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../rs/lnbmp/c_interface bmp_c_interface)
