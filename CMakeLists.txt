
set(CMAKE_POLICY_DEFAULT_CMP0077 NEW)

cmake_minimum_required(VERSION 3.0)
MESSAGE(STATUS "======================")
MESSAGE(STATUS "Starting usb test")
MESSAGE(STATUS "======================")

SET(LN_ENABLE_USBD True CACHE INTERNAL "")
SET(LN_ENABLE_I2C  FALSE CACHE INTERNAL "")
SET(LN_ENABLE_SPI  FALSE CACHE INTERNAL "")

SET(AF_FOLDER  ${CMAKE_SOURCE_DIR}/lnArduino/)
include(./mcuSelect.cmake)
# Update paths
SET(CMAKE_TOOLCHAIN_FILE ${AF_FOLDER}/lnArduino.cmake)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${AF_FOLDER}/cmake CACHE INTERNAL "")

PROJECT(lnBMP NONE) # the NONE is very important !

enable_language(C CXX ASM) # this is important too!
#

include(applyPatch)
APPLY_PATCH_IF_NEEDED(patched1 blackmagic_add_ch32_support.patch       blackmagic         "add ch32f103 support")
APPLY_PATCH_IF_NEEDED(patched2 blackmagic_dont_overuse_the_stack.patch blackmagic         "dont overuse the stack ")
APPLY_PATCH_IF_NEEDED(patched3 blackmagic_dontcopy_partial_reply.patch blackmagic         "avoid copy  ")
APPLY_PATCH_IF_NEEDED(patched4 blackmagic_reuse_exception.patch        blackmagic         "reuse exception to lower stack usage ")
APPLY_PATCH_IF_NEEDED(patched5 blackmagic_add_gdb_putpacket2.patch     blackmagic         "add gdb_putpacket2 to send a packet made of two chunks")
APPLY_PATCH_IF_NEEDED(patched6 blackmagic_small_exception.patch        blackmagic         "use smaller setjmp/long jmp on smaller arm")
#

include_directories(${CMAKE_SOURCE_DIR})
include_directories(${CMAKE_SOURCE_DIR}/include)

#SET(LN_ENABLE_RUST ON CACHED "")

add_subdirectory(${AF_FOLDER})

# tiny USB configuration
SET(LN_USB_NB_CDC 2) # 2 CDC interfaces
SET(LN_USB_NB_HID 0) # No HID
include(${AF_FOLDER}/setup.cmake)
USE_LIBRARY(tinyUsb)

include(${AF_FOLDER}/libraries/tinyUsb/tiny.cmake)

GENERATE_GD32_FIRMWARE(lnBMP src/lnBMP.cpp)
ADD_SUBDIRECTORY(lnBlackmagic)
target_link_libraries(lnBMP lnBlackmagic gd32_usb_usbd)
