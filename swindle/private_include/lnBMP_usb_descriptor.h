#pragma once
#include "dfu/dfu.h"
#define _PID_MAP(itf, n) ((CFG_TUD_##itf) << (n))

#define USB_VID 0x1d50

#define USB_BCD 0x0200

/**
 *
 */
// RP2040
#if defined(USE_RP2040) || defined(USE_RP2350)
#define USB_PID 0x6050
const char *descriptor[] = {
    (const char[]){0x09, 0x04},                     // 0: is supported language is English (0x0409)
    "picolnSwindle",                                // 1: Manufacturer
    "Swindle, Custom implementation of BMP (pico)", // 2: Product
    "45678",                                        // 3: Serials, should use chip ID
    "pswindle GDB Server",                          // 4: CDC Interface
    "pswindle GDB Uart",                            // 5: CDC Interface
    "pswindle GDB Log",                             // 5: CDC Interface
    "pswindle DFU",                                 // 6: DFU Interface
};
#include "lnBMP_usb_descriptor_3cdc.h"
#else
#define USB_PID 0x6030
const char *descriptor[] = {
    (const char[]){0x09, 0x04},              // 0: is supported language is English (0x0409)
    "swindle",                               // 1: Manufacturer
    "Swindle, Custom implementation of BMP", // 2: Product
    "45678",                                 // 3: Serials, should use chip ID
    "swindle GDB Server",                    // 4: CDC Interface
    "swindle GDB Uart",                      // 5: CDC Interface
    "swindle DFU",                           // 7: DFU Interface
};
#include "lnBMP_usb_descriptor_2cdc.h"
#endif

// device qualifier is mostly similar to device descriptor since we don't change configuration based on speed
const tusb_desc_device_qualifier_t desc_device_qualifier = {.bLength = sizeof(tusb_desc_device_t),
                                                            .bDescriptorType = TUSB_DESC_DEVICE,
                                                            .bcdUSB = USB_BCD,

                                                            .bDeviceClass = TUSB_CLASS_MISC,
                                                            .bDeviceSubClass = MISC_SUBCLASS_COMMON,
                                                            .bDeviceProtocol = MISC_PROTOCOL_IAD,

                                                            .bMaxPacketSize0 = CFG_TUD_ENDPOINT0_SIZE,
                                                            .bNumConfigurations = 0x01,
                                                            .bReserved = 0x00};
