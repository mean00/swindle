/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of blackmagic/src/target/stm32f4.c, made for the Swindle firmware.
 */

#![no_std]

//! Pure Rust driver for the STM32F2/F4/F7 and GD32F4 families (F4-style FPEC).

pub mod driver;
pub mod flash;
pub mod mon_commands;
pub mod registers;

pub use driver::{F4Driver, probe_gd32, probe_stm32};
