/*
  lnBMP:  Completely disable jtag to gain a bit of flash real estate
Original license header

 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.


 This file implements the SW-DP interface.

 */
 #include "lnArduino.h"
 #include "lnBMP_pinout.h"
 extern "C"
 {

#include "general.h"
#include "timing.h"
#include "adiv5.h"
#include "jtagtap.h"
#include "exception.h"


extern jtag_proc_t jtag_proc={NULL,NULL,NULL,NULL,NULL};
 int jtagtap_init()
{
  return 0;
}
int jtag_scan(const uint8_t *irlens)
{
  return -1;
}
void adiv5_jtagdp_abort(ADIv5_DP_t *dp, uint32_t abort)
{

}
uint32_t fw_adiv5_jtagdp_read(ADIv5_DP_t *dp, uint16_t addr)
{
  return 0;
}
//uint32_t fw_adiv5_jtagdp_read(ADIv5_DP_t *dp, uint16_t addr)
//{
//  return 0;
//}
uint32_t fw_adiv5_jtagdp_low_access(ADIv5_DP_t *dp, uint8_t RnW,
                     uint16_t addr, uint32_t value)
 {
    raise_exception(EXCEPTION_ERROR, "JTAG-DP disabled");
   return 0; //
 }

}
