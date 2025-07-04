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
#define MKNAME(x) "p"##x
#else
#define USB_PID 0x6030
#define MKNAME(x) x
#endif
const char *descriptor[] = {
    (const char[]){0x09, 0x04},   // 0: is supported language is English (0x0409)
    MKNAME("Swindle"),            // 1: Manufacturer
    MKNAME("Swindle debugger "),  // 2: Product
    "45678",                      // 3: Serials, should use chip ID
    MKNAME("swindle GDB Server"), // 4: CDC Interface
    MKNAME("swindle GDB Uart"),   // 5: CDC Interface
#ifdef USE_3_CDC
    MKNAME("swindle GDB Log"), // 5: CDC Interface
#endif
    MKNAME("swindle DFU"), // 6: DFU Interface
};
#ifdef USE_3_CDC
#include "lnBMP_usb_descriptor_3cdc.h"
#else
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
