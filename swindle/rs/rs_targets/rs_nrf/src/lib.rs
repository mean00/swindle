#![no_std]

//! Pure Rust drivers for Nordic Semiconductor targets:
//! - nRF51 / nRF52 series (`Nrf51Driver`)
//! - nRF54L series (`Nrf54lDriver`)
//! - nRF91 series (`Nrf91Driver`)
//! - CTRL-AP recovery access port drivers (`ctrl_ap`)

pub mod ctrl_ap;
pub mod mon_commands;
pub mod nrf51;
pub mod nrf54l;
pub mod nrf91;
pub mod registers;

pub use ctrl_ap::{nrf51_ctrl_ap_probe, nrf54l_ctrl_ap_probe};
pub use nrf51::Nrf51Driver;
pub use nrf54l::Nrf54lDriver;
pub use nrf91::Nrf91Driver;
