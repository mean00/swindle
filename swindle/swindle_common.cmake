SET(S ${BMP}/src CACHE INTERNAL "")
SET(T ${BMP}/src/target CACHE INTERNAL "")
SET(P ${BMP}/src/platforms CACHE INTERNAL "")
SET(myB ${CMAKE_CURRENT_SOURCE_DIR}/ CACHE INTERNAL "")
SET(B ${myB}/src CACHE INTERNAL "")
ADD_DEFINITIONS("-DENABLE_RISCV")
ADD_DEFINITIONS("-DCONFIG_RISCV")
ADD_DEFINITIONS("-DCONFIG_GD32")
ADD_DEFINITIONS("-DCONFIG_MM32")
ADD_DEFINITIONS("-DCONFIG_RVSWD -DPLATFORM_HAS_RVSWD")

#ADD_DEFINITIONS("-DENABLE_RTT")
ADD_DEFINITIONS("-DNO_LIBOPENCM3=1")
#----------------------------
# Supported boards
#----------------------------

SET(BMP_EXTRA  ${B}/../../blackmagic_addon/)
SET(BOARDS      ${enabled_sources}
                ${BMP_EXTRA}/target/CH32V3xx/ch32v3xx.c
                ${B}/bmp_disabledBoard.cpp
                ${T}/lpc_common.c
                CACHE INTERNAL ""
        )

#-------
SET(BM_SRC      ${S}/command.c
                ${S}/exception.c
                ${S}/hex_utils.c
                ${S}/timing.c
                ${S}/maths_utils.c
                #${S}/rtt.c
                #${S}/remote.c
                #${S}/gdb_main.c
                #${S}/gdb_packet.c
                #${S}/crc32.c
                #${S}/morse.c
                CACHE INTERNAL ""
                )
#------
SET(BM_TARGET
                ${T}/adi.c
                ${T}/adiv5.c
                ${T}/adiv5_swd.c
                ${T}/adiv6.c
                ${T}/cortex.c
                ${T}/cortexm.c
                ${T}/riscv_adi_dtm.c
                ${T}/riscv_debug.c
                ${T}/riscv32.c
                ${T}/target_flash.c
                ${T}/gdb_reg.c
                ${T}/target.c
                ${T}/sfdp.c
                ${T}/spi.c
                ${T}/stm32_common.c
                        #${T}/riscv64.c
                        #${S}/gdb_hostio.c
                        #${T}/jtag_scan.c) ${T}/jtag_devs.c    ${T}/adiv5_jtagdp.c
                        #${T}/cortexa.c
                        #${T}/lmi.c
                        #${T}/target_probe.c

                CACHE INTERNAL ""
                )
include_directories( ${BMP}/src/include)
include_directories( ${BMP}/src)
include_directories( ${BMP}/src/target)
#---------


