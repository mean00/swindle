#include "lnArduino.h"

#include "include/lnUsbStack.h"
#include "include/lnUsbCDC.h"

#include "../lnBlackmagic/private_include/lnBMP_usb_descriptor.h"

#define LED LN_SYSTEM_LED
#define LED2 PA8

extern "C" void user_init();

extern "C" void rnLoop();

extern void lnusb_set_global_configuration(const uint8_t *hsConfiguration, const uint8_t *fsConfiguration,
                          const tusb_desc_device_t *desc, const tusb_desc_device_qualifier_t *qual);


/**
 */
void setup()
{
    lnPinMode(LED, lnOUTPUT);
    lnPinMode(LED2, lnOUTPUT);
}
void loop()
{
    Logger("Starting lnBMP Test\n");
    user_init();
    // Init C side of USB
    lnusb_set_global_configuration(desc_hs_configuration,   desc_fs_configuration,
                          &desc_device, &desc_device_qualifier);

    rnLoop();
    while (1)
    {
        // Logger("*\n");
        delay(1000);
        lnDigitalToggle(LED);
        lnDigitalToggle(LED2);
    }
}

void gdb_task(void *)
{
    xAssert(0);
}