

SET(S ${BMP}/src)
SET(T ${BMP}/src/target )
SET(P ${BMP}/src/platforms )

SET(myB ${CMAKE_CURRENT_SOURCE_DIR}/ )

SET(B ${myB}/src )

OPTION(LN_BMP_HOSTED  "Enable Hosted mode" OFF)


# hre are the boards that are removed, dont forget to edit bmp_disabledBoard if you change that list
# ${T}/lpc15xx.c ${T}/lpc43xx.c   ${T}/lpc_common.c ${T}/nrf51.c ${T}/rp.c ${T}/sam4l.c ${T}/samx5x.c  ${T}/kinetis.c${T}/samd.c
# ${T}/stm32f1.c ${T}/stm32g0.c   ${T}/stm32l0.c
# ${T}/stm32f4.c ${T}/stm32h7.c ${T}/stm32l4.c  ${T}/efm32.c   ${T}/lpc11xx.c ${T}/lpc17xx.c ${T}/lpc546xx.c ${T}/msp432.c
# ${T}/nxpke04.c ${T}/sam3x.c
# dont need cortexa ${T}/cortexa.c
# The one i use...
SET(BOARDS  ${T}/lpc_common.c   ${T}/stm32f1.c ${T}/stm32g0.c  ${T}/lpc11xx.c ${B}/bmp_disabledBoard.cpp)

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
    SET(PC ${BMP}/src/platforms/pc )

    include_directories( ${HOSTED})
    SET(BM_HOSTED
            ${HOSTED}/bmp_remote.c ${HOSTED}/bmp_serial.c      ${HOSTED}/dap.c
            ${HOSTED}/platform.c   ${HOSTED}/remote_jtagtap.c  ${HOSTED}/remote_swdptap.c
            ${PC}/cl_utils.c   ${PC}/gdb_if.c    ${PC}/serial_unix.c  ${PC}/utils.c
            )
ELSE()  # "Normal" /Standalone mode
  MESSAGE(STATUS "Building for embedded mode (${LN_BMP_HOSTED})")
  ADD_DEFINITIONS("-DPLATFORM_IDENT=\"lnBMP\"")
  ADD_DEFINITIONS("-DPC_HOSTED=0")
  include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)
  #  add lnBlackmagic target code and host connection stuff
  SET(BRIDGE_SRCS ${B}/bridge.cpp ${B}/bmp_gpio.cpp  ${B}/bmp_stubs.cpp
          ${B}/bmp_swdTap.cpp   ${B}/bmp_serial.cpp  ${B}/bmp_gdb.cpp
          )
  include_directories( ${CMAKE_SOURCE_DIR}/lnBlackmagic/include )
ENDIF()
SET(BM_SRC          ${S}/command.c  ${S}/crc32.c        ${S}/exception.c  ${S}/gdb_hostio.c   ${S}/gdb_main.c ${S}/gdb_packet.c
                      ${S}/hex_utils.c ${S}/morse.c ${S}/remote.c ${S}/timing.c)
SET(BM_TARGET       ${T}/adiv5.c    ${T}/adiv5_swdp.c   ${T}/cortexm.c    ${T}/jtag_devs.c    ${T}/lmi.c      ${T}/target.c
                      ${T}/adiv5_jtagdp.c  ${T}/jtag_scan.c)

SET(BM_PLATFORM_COMMON   ${B}/bmp_freertos.cpp ${B}/bmp_string.cpp ${B}/bmp_utils.cpp ${B}/bmp_jtagstubs.cpp  )

add_library(lnBlackmagic STATIC ${BM_SRC} ${BRIDGE_SRCS} ${BM_TARGET} ${BM_HOSTED} ${BM_PLATFORM_COMMON} ${BOARDS})

target_include_directories(lnBlackmagic PRIVATE  ${S}/include ${B}/include ${T} ${CMAKE_BINARY_DIR}/config )
target_include_directories(lnBlackmagic PRIVATE  ${myB}/private_include)
