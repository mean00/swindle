/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of blackmagic/src/target/stm32f1.c, made for the Swindle firmware.
 */

#![no_std]

//! Pure Rust drivers for ARM Cortex-M targets whose flash controller is the
//! STM32F1-style FPEC (STM32F1/F0/F3 and the GD32F1/F2/F3/E230/E5 clones).

pub mod crc32;
pub mod families;
pub mod flash;
pub mod flashstub_crc32;
pub mod flashstub_write;
pub mod flashstub_write16;
pub mod flashstub_write32;
pub mod mon_commands;
pub mod registers;
pub mod stm32_parts;

pub use families::f1::F1Driver;
