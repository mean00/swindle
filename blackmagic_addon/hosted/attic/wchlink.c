/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#include <libusb.h>
#include "bmp_hosted.h"
#include "wchlink.h"

#include "riscv_debug.h"
#include "riscv_dtm_wchlink.h"

#define WCHLINK_MODERV_OP_EPT_ADDR    0x1U
#define WCHLINK_MODERV_FLASH_EPT_ADDR 0x2U

#define TRANSFER_TIMEOUT_MS 100U

#define WCH_CMD_OUT_HEADER 0x81U
#define WCH_CMD_IN_HEADER  0x82U

#define WCH_CMD_UNKNOWN_FIELD     0x01U
#define WCH_CMD_UNKNOWN_FIELD_DMI 0x06U

/* Flash commands */
#define WCH_FLASH_GRP       0x02U
#define WCH_FLASH_CHIPERASE 0x01U /* Unconfirmed */
#define WCH_FLASH_TERMINATE 0x08U /* Unconfirmed */

/* Flash potection commands */
#define WCH_PROTECT_GRP                 0x06U
#define WCH_PROTECT_CMD_CHECK           0x01U
#define WCH_PROTECT_CMD_FLASH_UNPROTECT 0x02U
#define WCH_PROTECT_CMD_FLASH_PROTECT   0x03U

/* DMI access commands */
#define WCH_DMI_GRP       0x08U
#define WCH_DMI_CMD_NOP   0x00U
#define WCH_DMI_CMD_READ  0x01U
#define WCH_DMI_CMD_WRITE 0x02U

/* System commands */
#define WCH_SYS_GRP             0x0dU
#define WCH_SYS_CMD_GET_VERSION 0x01U
#define WCH_SYS_CMD_CONNECT     0x02U
#define WCH_SYS_CMD_UNKNOWN     0x03U /* Unknown - no clue */
#define WCH_SYS_CMD_GET_MEMINFO 0x04U /* RAM size, flash size, addr. */
#define WCH_SYS_CMD_CLOSE       0xffU /* Terminate connection (unsure what this does) */

/* Debug commands */
#define WCH_DBG_GRP             0x0eU
#define WCH_DBG_CMD_DISABLE_DBG 0x01U

/* Error ? who knows */
#define WCH_ERR_GRP 0x55U

static bool wchlink_bulk_write(bmda_probe_s *const info, uint8_t *const data, const size_t length)
{
	int transferred = 0;
	const int result = libusb_bulk_transfer(
		info->usb_link->device_handle, info->usb_link->ep_tx, data, length, &transferred, TRANSFER_TIMEOUT_MS);
	if (result < 0) {
		DEBUG_WARN("WCH-Link write error: %s (%d)\n", libusb_strerror(result), result);
		return false;
	}

	return true;
}

static bool wchlink_bulk_read(bmda_probe_s *const info, uint8_t *const data, const size_t length)
{
	int transferred = 0;
	const int result = libusb_bulk_transfer(
		info->usb_link->device_handle, info->usb_link->ep_rx, data, length, &transferred, TRANSFER_TIMEOUT_MS);
	if (result < 0) {
		DEBUG_WARN("WCH-Link read error: %s (%d)\n", libusb_strerror(result), result);
		return false;
	}

	return true;
}

/*
 * On success this copies the endpoint addresses identified into the
 * usb_link_s sub-structure of bmp_info_s (info->usb_link) for later use.
 * Returns true for success, false for failure.
 */
static bool claim_wchlink_interface(bmda_probe_s *info, libusb_device *dev)
{
	libusb_config_descriptor_s *config;
	const int result = libusb_get_active_config_descriptor(dev, &config);
	if (result != LIBUSB_SUCCESS) {
		DEBUG_WARN("Failed to get configuration descriptor: %s\n", libusb_error_name(result));
		return false;
	}

	const libusb_interface_descriptor_s *descriptor = NULL;
	for (size_t i = 0; i < config->bNumInterfaces; ++i) {
		const libusb_interface_s *const interface = &config->interface[i];
		const libusb_interface_descriptor_s *const interface_desc = &interface->altsetting[0];
		if (interface_desc->bInterfaceClass == LIBUSB_CLASS_VENDOR_SPEC && interface_desc->bInterfaceSubClass == 128U) {
			const int result = libusb_claim_interface(info->usb_link->device_handle, i);
			if (result) {
				DEBUG_WARN("Can not claim handle: %s\n", libusb_error_name(result));
				break;
			}

			info->usb_link->interface = i;
			descriptor = interface_desc;
		}
	}
	if (!descriptor) {
		DEBUG_WARN("No suitable interface found\n");
		libusb_free_config_descriptor(config);
		return false;
	}

	for (size_t i = 0; i < descriptor->bNumEndpoints; i++) {
		const libusb_endpoint_descriptor_s *endpoint = &descriptor->endpoint[i];

		if ((endpoint->bEndpointAddress & LIBUSB_ENDPOINT_ADDRESS_MASK) == WCHLINK_MODERV_OP_EPT_ADDR) {
			if (endpoint->bEndpointAddress & LIBUSB_ENDPOINT_DIR_MASK)
				info->usb_link->ep_rx = endpoint->bEndpointAddress;
			else
				info->usb_link->ep_tx = endpoint->bEndpointAddress;
		}
	}
	libusb_free_config_descriptor(config);
	return true;
}

static bool wchlink_read_version(bmda_probe_s *info)
{
	uint8_t cmd[] = {
		WCH_CMD_OUT_HEADER,
		WCH_SYS_GRP,
		WCH_CMD_UNKNOWN_FIELD,
		WCH_SYS_CMD_GET_VERSION,
	};
	if (!wchlink_bulk_write(info, cmd, sizeof(cmd)))
		return false;

	uint8_t response[7];
	if (!wchlink_bulk_read(info, response, sizeof(response)))
		return false;

	const uint8_t wchlink_major = response[3];
	const uint8_t wchlink_minor = response[4];
	const uint8_t wchlink_id = response[5];

	const char *wchlink_name = NULL;
	switch (wchlink_id) {
	case 1:
		wchlink_name = "WCH-Link-CH549 (RV)";
		break;
	case 2:
		wchlink_name = "WCH-LinkE-CH32V307 (RV)";
		break;
	case 3:
		wchlink_name = "WCH-LinkS-CH32V203 (RV)";
		break;
	case 4:
		wchlink_name = "WCH-LinkB (RV)";
		break;
	default:
		wchlink_name = "unknow WCH-LINK";
		break;
	}

	DEBUG_INFO("%s (id:%u) v%u.%u\n", wchlink_name, wchlink_id, wchlink_major, wchlink_minor);

	return true;
}

static bool wchlink_connect(bmda_probe_s *info, uint8_t *const device_code, uint32_t *const device_id)
{
	uint8_t cmd[] = {
		WCH_CMD_OUT_HEADER,
		WCH_SYS_GRP,
		WCH_CMD_UNKNOWN_FIELD,
		WCH_SYS_CMD_CONNECT,
	};
	if (!wchlink_bulk_write(info, cmd, sizeof(cmd)))
		return false;

	uint8_t response[8];
	if (!wchlink_bulk_read(info, response, sizeof(response)))
		return false;

	if (response[1] != WCH_SYS_GRP) { // response[1] returns WCH_ERR_GRP (0x55) on error
		DEBUG_WARN("WCH-Link failed to connect with device\n");
		return false;
	}

	*device_code = response[3];
	*device_id = (response[4] << 24U) + (response[5] << 16U) + (response[6] << 8U) + (response[7] & 0x0fU);

	return true;
}

static bool wchlink_unknown(bmda_probe_s *info)
{
	uint8_t cmd[] = {
		WCH_CMD_OUT_HEADER,
		WCH_SYS_GRP,
		WCH_CMD_UNKNOWN_FIELD,
		WCH_SYS_CMD_UNKNOWN,
	};
	if (!wchlink_bulk_write(info, cmd, sizeof(cmd)))
		return false;

	uint8_t response[4];
	if (!wchlink_bulk_read(info, response, sizeof(response)))
		return false;

	return true;
}

bool wchlink_riscv_dmi_read( const uint32_t address, uint32_t *const value)
{
	uint8_t cmd[] = {
		WCH_CMD_OUT_HEADER,
		WCH_DMI_GRP,
		WCH_CMD_UNKNOWN_FIELD_DMI,
		address & 0xffU,
		0U,
		0U,
		0U,
		0U,
		WCH_DMI_CMD_READ,
	};

	if (!wchlink_bulk_write(&bmda_probe_info, cmd, sizeof(cmd)))
		return false;

	uint8_t response[9];
	if (!wchlink_bulk_read(&bmda_probe_info, response, sizeof(response)))
		return false;

	if (response[8]) { /* status */
		DEBUG_WARN("WCH-Link failed to read DMI register\n");
		return false;
	}

	*value = (response[4] << 24U) + (response[5] << 16U) + (response[6] << 8U) + response[7];

	return true;
}

bool wchlink_riscv_dmi_write( const uint32_t address, const uint32_t value)
{
	uint8_t cmd[] = {
		WCH_CMD_OUT_HEADER,
		WCH_DMI_GRP,
		WCH_CMD_UNKNOWN_FIELD_DMI,
		address & 0xffU,
		value >> 24U,
		value >> 16U,
		value >> 8U,
		value & 0xffU,
		WCH_DMI_CMD_WRITE,
	};

	if (!wchlink_bulk_write(&bmda_probe_info, cmd, sizeof(cmd)))
		return false;

	uint8_t response[9];
	if (!wchlink_bulk_read(&bmda_probe_info, response, sizeof(response)))
		return false;

	if (response[8]) { /* status */
		DEBUG_WARN("WCH-Link failed to read DMI register\n");
		return false;
	}

	return true;
}

static bool wchlink_init2(bmda_probe_s *const info)
{
	usb_link_s *link = calloc(1, sizeof(usb_link_s));
	if (!link)
		return false;
	info->usb_link = link;
	link->context = info->libusb_ctx;

	libusb_device **device_list = NULL;
	const ssize_t device_count = libusb_get_device_list(info->libusb_ctx, &device_list);
	if (device_count < 0) {
		DEBUG_WARN("libusb_get_device_list() failed");
		return false;
	}

	libusb_device *device_wchlink = NULL;
	for (ssize_t device_index = 0; device_index < device_count; ++device_index) {
		if (!device_list[device_index])
			continue;

		libusb_device *const device = device_list[device_index];
		struct libusb_device_descriptor device_descriptor;
		if (libusb_get_device_descriptor(device, &device_descriptor) < 0) {
			DEBUG_WARN("libusb_get_device_descriptor() failed");
			libusb_free_device_list(device_list, 1);
			return false;
		}

		if (device_descriptor.idVendor != info->vid || device_descriptor.idProduct != info->pid)
			continue;

		int result = libusb_open(device, &link->device_handle);
		if (result != LIBUSB_SUCCESS)
			continue;

		device_wchlink = device;
		break;
	}

	if (!device_wchlink || !claim_wchlink_interface(info, device_wchlink)) {
		libusb_free_device_list(device_list, 1);
		return false;
	}

	if (!link->ep_tx || !link->ep_rx) {
		DEBUG_WARN("Device setup failed\n");
		libusb_release_interface(info->usb_link->device_handle, info->usb_link->interface);
		libusb_close(info->usb_link->device_handle);
		libusb_free_device_list(device_list, 1);
		return false;
	}

	libusb_free_device_list(device_list, 1);

	return wchlink_read_version(info);
}

bool wchlink_init(bmda_probe_s *const info)
{
	return wchlink_init2(&bmda_probe_info);

}

const char *wchlink_target_voltage()
{	
	return "Unavailable";
}

void wchlink_nrst_set_val( bool assert)
{
	(void)assert;
}

bool wchlink_nrst_get_val()
{	
	return true;
}

uint32_t wchlink_rvswd_scan()
{

	target_list_free();

	uint8_t device_code;
	uint32_t device_id;

	if (!wchlink_connect(&bmda_probe_info, &device_code, &device_id)) {
		DEBUG_WARN("WCH-Link failed to connect to target");
		return 0U;
	}

	DEBUG_WARN("WCH-Link connected with 0x%0X\n", device_id);

	switch (device_code) {
	case 0x1U:
	case 0x5U:
	case 0x6U:
	case 0x9U:
	case 0xaU:
		wchlink_unknown(&bmda_probe_info);
		break;

	default:
		break;
	}

	return riscv_dtm_wchlink_handler();
}
