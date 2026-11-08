//! `mon` commands for Nordic Semiconductor targets (nRF51 / nRF52).

use crate::registers::*;
use rsbmp::commands::{CallbackType, CommandTree};
use rsbmp::encoder::encoder;
use rsbmp::gdb_println;
use rsbmp::target::{CortexMAccess, get_cur_target};

pub const COMMANDS: [CommandTree; 3] = [
    CommandTree {
        command: "erase_uicr",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_erase_uicr),
        start_separator: b' ',
        next_separator: b' ',
        help: "Erase UICR registers",
    },
    CommandTree {
        command: "protect_flash",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_protect_flash),
        start_separator: b' ',
        next_separator: b' ',
        help: "Enable flash read/write protection",
    },
    CommandTree {
        command: "read",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_read),
        start_separator: b' ',
        next_separator: b' ',
        help: "Read device parameters",
    },
];

fn wait_ready(cm: &CortexMAccess) -> bool {
    let mut count = 0;
    while cm.read32(nvmc_reg!(ready)) == 0 {
        if cm.check_error() {
            return false;
        }
        count += 1;
        if count > 1_000_000 {
            return false;
        }
    }
    true
}

fn _erase_uicr(_cmd: &str, _args: &[&str]) -> bool {
    let target = get_cur_target();
    if target.is_null() {
        return false;
    }
    let cm = CortexMAccess::new(target);

    gdb_println!("Erasing..\n");
    cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_EEN);
    if !wait_ready(&cm) {
        return false;
    }

    cm.write32(nvmc_reg!(eraseuicr), 1);
    if !wait_ready(&cm) {
        return false;
    }

    cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_REN);
    let _ = wait_ready(&cm);

    encoder::reply_ok();
    true
}

fn _protect_flash(_cmd: &str, _args: &[&str]) -> bool {
    let target = get_cur_target();
    if target.is_null() {
        return false;
    }
    let cm = CortexMAccess::new(target);

    gdb_println!("Enabling Flash protection..\n");
    cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_WEN);
    if !wait_ready(&cm) {
        return false;
    }

    cm.write32(uicr_reg!(approtect), 0xffff_ff00);
    if !wait_ready(&cm) {
        return false;
    }

    cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_REN);
    let _ = wait_ready(&cm);

    encoder::reply_ok();
    true
}

fn _read(_cmd: &str, args: &[&str]) -> bool {
    let target = get_cur_target();
    if target.is_null() {
        return false;
    }
    let cm = CortexMAccess::new(target);

    let subcmd = args.first().copied().unwrap_or("");
    match subcmd {
        "hwid" => {
            let hwid = cm.read32(ficr_reg!(configid)) & 0xffff;
            gdb_println!("Hardware ID: 0x{:04X}\n", hwid);
            encoder::reply_ok();
            true
        }
        "fwid" => {
            let fwid = (cm.read32(ficr_reg!(configid)) >> 16) & 0xffff;
            gdb_println!("Firmware ID: 0x{:04X}\n", fwid);
            encoder::reply_ok();
            true
        }
        "deviceid" => {
            let low = cm.read32(ficr_reg!(deviceid_low));
            let high = cm.read32(ficr_reg!(deviceid_high));
            gdb_println!("Device ID: 0x{:08X}{:08X}\n", high, low);
            encoder::reply_ok();
            true
        }
        "deviceaddr" => {
            let addr_type = cm.read32(ficr_reg!(deviceaddrtype));
            let low = cm.read32(ficr_reg!(deviceaddr_low));
            let high = cm.read32(ficr_reg!(deviceaddr_high)) & 0xffff;
            if (addr_type & 1) == 0 {
                gdb_println!("Publicly Listed Address: 0x{:04X}{:08X}\n", high, low);
            } else {
                gdb_println!("Randomly Assigned Address: 0x{:04X}{:08X}\n", high, low);
            }
            encoder::reply_ok();
            true
        }
        "deviceinfo" => {
            let part = cm.read32(ficr_reg!(info_part));
            let variant = cm.read32(ficr_reg!(info_variant)).to_le_bytes();
            let package = cm.read32(ficr_reg!(info_package));
            let ram = cm.read32(ficr_reg!(info_ram));
            let flash = cm.read32(ficr_reg!(info_flash));

            gdb_println!("Part:\t\tNRF{:X}\n", part);
            gdb_println!(
                "Variant:\t{}{}{}{}\n",
                variant[3] as char,
                variant[2] as char,
                variant[1] as char,
                variant[0] as char
            );
            match package {
                NRF51_FIELD_UNSPECIFIED => gdb_println!("Package:\tUnspecified\n"),
                0x2000 => gdb_println!("Package:\tQF\n"),
                0x2001 => gdb_println!("Package:\tCI\n"),
                0x2004 => gdb_println!("Package:\tQIxx\n"),
                other => gdb_println!("Package:\tUnknown (Code {:X})\n", other),
            }
            gdb_println!("Ram:\t\t{}kiB\n", ram);
            gdb_println!("Flash:\t\t{}kiB\n", flash);
            encoder::reply_ok();
            true
        }
        _ => {
            gdb_println!("Read commands:\n");
            gdb_println!("\thelp -- Display help for read commands\n");
            gdb_println!("\thwid -- Read hardware identification number\n");
            gdb_println!("\tfwid -- Read pre-loaded firmware ID\n");
            gdb_println!("\tdeviceid -- Print unique device ID\n");
            gdb_println!("\tdeviceaddr -- Read device address\n");
            gdb_println!("\tdeviceinfo -- Read device information\n");
            encoder::reply_ok();
            true
        }
    }
}
