#pragma once
#include "dfu/dfu.h"
#define _PID_MAP(itf, n) ((CFG_TUD_##itf) << (n))

#define USB_VID 0x1d50
#define USB_PID 0x6040

#define USB_BCD 0x0200

/**
 *
 */
// RP2040
#ifdef USE_RP2040
const char *descriptor[] = {
    (const char[]){0x09, 0x04},     // 0: is supported language is English (0x0409)
    "picolnBlackMagic",             // 1: Manufacturer
    "Custom implementation of BMP (pico)", // 2: Product
    "45678",                        // 3: Serials, should use chip ID
    "plnBMP GDB Server",            // 4: CDC Interface
    "plnBMP GDB Uart",              // 5: CDC Interface
    "plnBMP DFU",                   // 6: DFU Interface
};
#else
const char *descriptor[] = {
    (const char[]){0x09, 0x04},     // 0: is supported language is English (0x0409)
    "lnBlackMagic",                 // 1: Manufacturer
    "Custom implementation of BMP", // 2: Product
    "45678",                        // 3: Serials, should use chip ID
    "lnBMP GDB Server",             // 4: CDC Interface
    "lnBMP GDB Uart",               // 5: CDC Interface
    "lnBMP DFU",                    // 6: DFU Interface
};
#endif
const tusb_desc_device_t desc_device = {
    .bLength = sizeof(tusb_desc_device_t),
    .bDescriptorType = TUSB_DESC_DEVICE,
    .bcdUSB = USB_BCD,

    // Use Interface Association Descriptor (IAD) for CDC
    // As required by USB Specs IAD's subclass must be common class (2) and protocol must be IAD (1)
    .bDeviceClass = TUSB_CLASS_MISC,
    .bDeviceSubClass = MISC_SUBCLASS_COMMON,
    .bDeviceProtocol = MISC_PROTOCOL_IAD,
    .bMaxPacketSize0 = CFG_TUD_ENDPOINT0_SIZE,

    .idVendor = USB_VID,
    .idProduct = USB_PID,
    .bcdDevice = 0x0100,

    .iManufacturer = 0x01,
    .iProduct = 0x02,
    .iSerialNumber = 0x03,

    .bNumConfigurations = 0x01};

//--------------------------------------------------------------------+
// Configuration Descriptor
//--------------------------------------------------------------------+
enum
{
    ITF_NUM_CDC_0 = 0,
    ITF_NUM_CDC_0_DATA,
    ITF_NUM_CDC_1,
    ITF_NUM_CDC_1_DATA,
    ITF_NUM_DFU_RT,
    ITF_NUM_TOTAL
};

#define CONFIG_TOTAL_LEN (TUD_CONFIG_DESC_LEN + (CFG_TUD_CDC * TUD_CDC_DESC_LEN) + TUD_DFU_RT_DESC_LEN)
#define EPNUM_CDC_0_NOTIF 0x81
#define EPNUM_CDC_0_OUT 0x02
#define EPNUM_CDC_0_IN 0x82

#define EPNUM_CDC_1_NOTIF 0x83
#define EPNUM_CDC_1_OUT 0x04
#define EPNUM_CDC_1_IN 0x84

const uint8_t desc_fs_configuration[] = {
    // Config number, interface count, string index, total length, attribute, power in mA
    TUD_CONFIG_DESCRIPTOR(1, ITF_NUM_TOTAL, 0, CONFIG_TOTAL_LEN, 0x00, 100),

    // 1st CDC: Interface number, string index, EP notification address and size, EP data address (out, in) and size.
    TUD_CDC_DESCRIPTOR(ITF_NUM_CDC_0, 4, EPNUM_CDC_0_NOTIF, 8, EPNUM_CDC_0_OUT, EPNUM_CDC_0_IN, 64),

    // 2nd CDC: Interface number, string index, EP notification address and size, EP data address (out, in) and size.
    TUD_CDC_DESCRIPTOR(ITF_NUM_CDC_1, 5, EPNUM_CDC_1_NOTIF, 8, EPNUM_CDC_1_OUT, EPNUM_CDC_1_IN, 64),
    TUD_DFU_RT_DESCRIPTOR(ITF_NUM_DFU_RT, 6, 0x0d, 1000, 4096),

};

// Per USB specs: high speed capable device must report device_qualifier and other_speed_configuration

const uint8_t desc_hs_configuration[] = {
    // Config number, interface count, string index, total length, attribute, power in mA
    TUD_CONFIG_DESCRIPTOR(1, ITF_NUM_TOTAL, 0, CONFIG_TOTAL_LEN, 0x00, 100),

    // 1st CDC: Interface number, string index, EP notification address and size, EP data address (out, in) and size.
    TUD_CDC_DESCRIPTOR(ITF_NUM_CDC_0, 4, EPNUM_CDC_0_NOTIF, 8, EPNUM_CDC_0_OUT, EPNUM_CDC_0_IN, 512),

    // 2nd CDC: Interface number, string index, EP notification address and size, EP data address (out, in) and size.
    TUD_CDC_DESCRIPTOR(ITF_NUM_CDC_1, 5, EPNUM_CDC_1_NOTIF, 8, EPNUM_CDC_1_OUT, EPNUM_CDC_1_IN, 512),
    TUD_DFU_RT_DESCRIPTOR(ITF_NUM_DFU_RT, 6, 0x0d, 1000, 4096),
};

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
