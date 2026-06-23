//! Persistent key-value settings store.
//!
//! Provides a simple string→u32 mapping backed by a `heapless::LinearMap`.
//! Used to persist configuration values (RTT addresses, polling periods, etc.)
//! across GDB sessions. The settings are stored in RAM and initialised on
//! first access.
//!
//! ## C interface
//!
//! Exposes `bmp_settings_set`, `bmp_settings_get_or_default`, and
//! `bmp_settings_unset` for use from C code.

use crate::setting_keys::MAX_SETTING_ENTRIES;
use crate::setting_keys::SW_TOKEN_SIZE;
use arraystring::{ArrayString, typenum::U32};
use core::mem::MaybeUninit;
use heapless::LinearMap;
type token = ArrayString<U32>;
//
//setup_log!(false);
//use crate::gdb_print;
//crate::gdb_print_init!();

/// Internal settings storage: a fixed-capacity hash map.
struct swindle_settings {
    hash: LinearMap<token, u32, MAX_SETTING_ENTRIES>,
}
// SAFETY: swindle_settings is only used in a single-threaded debugger context.
unsafe impl Sync for swindle_settings {}
impl swindle_settings {
    /// Create an empty settings store.
    fn new() -> Self {
        swindle_settings {
            hash: LinearMap::new(),
        }
    }
    /// Clear all settings.
    #[allow(dead_code)]
    fn clear(&mut self) {
        self.hash.clear();
    }
}

//--

fn get_settings() -> &'static mut swindle_settings {
    static mut SETTINGS: MaybeUninit<swindle_settings> = MaybeUninit::uninit();
    static mut SETTINGS_INIT: bool = false;
    unsafe {
        if !SETTINGS_INIT {
            SETTINGS.write(swindle_settings::new());
            SETTINGS_INIT = true;
        }
        SETTINGS.assume_init_mut()
    }
}
/// Print all settings to the GDB console.
pub fn dump() {
    let info = get_settings();
    gdb_print!("Dumping settings \n");
    for (key, value) in &info.hash {
        gdb_println!("Key: ", key.as_str());
        gdb_print!("Value:  ", *value);
        gdb_println!("- Hex : ", Hex(*value as usize));
    }
}
/// Convert a string key to a fixed-size token, truncating if necessary.
fn to_token(key: &str) -> token {
    if key.len() <= SW_TOKEN_SIZE {
        let t: token = token::from(key);
        return t;
    }
    token::from(&key[..SW_TOKEN_SIZE])
}

/// Set a setting value by key.
pub fn set(k: &str, value: u32) {
    let info = get_settings();
    let key: token = to_token(k);
    if let Some(old_value) = info.hash.get_mut(&key) {
        *old_value = value;
    } else {
        let _ = info.hash.insert(key, value);
    }
}
/// Remove a setting by key. Returns `true` if the key existed.
pub fn remove(k: &str) -> bool {
    let info = get_settings();
    let key: token = to_token(k);
    if info.hash.contains_key(&key) {
        info.hash.remove(&key);
        return true;
    }
    false
}
/// Get a setting value, returning `def` if the key is not found.
pub fn get_or_default(k: &str, def: u32) -> u32 {
    let info = get_settings();
    let key: token = to_token(k);
    match info.hash.get(&key) {
        Some(x) => *x,
        None => def,
    }
}
/// Force initialisation of the settings store.
///
/// Settings are lazily initialised on first access; this function
/// ensures they are ready before any other code runs.
pub fn init_settings() {
    // OnceLock initialises lazily on first access; this call forces it.
    get_settings();
}
// C interface

#[cfg(feature = "hosted")]
type my_c_str = *const i8;
#[cfg(not(feature = "hosted"))]
type my_c_str = *const u8;

/// Set a setting from C code (C-compatible entry point).
#[unsafe(no_mangle)]
extern "C" fn bmp_settings_set(key: my_c_str, value: u32) -> bool {
    let c_str = unsafe { core::ffi::CStr::from_ptr(key) };
    match c_str.to_str() {
        Ok(x) => {
            set(x, value);
            true
        }
        Err(_) => false,
    }
}
//
/// Get a setting from C code, with a default value (C-compatible entry point).
#[unsafe(no_mangle)]
extern "C" fn bmp_settings_get_or_default(key: my_c_str, def: u32) -> u32 {
    let c_str = unsafe { core::ffi::CStr::from_ptr(key) };
    match c_str.to_str() {
        Ok(x) => get_or_default(x, def),
        Err(_) => 0,
    }
}
//
/// Remove a setting from C code (C-compatible entry point).
#[unsafe(no_mangle)]
extern "C" fn bmp_settings_unset(key: my_c_str) -> bool {
    let c_str = unsafe { core::ffi::CStr::from_ptr(key) };
    match c_str.to_str() {
        Ok(x) => remove(x),
        Err(_) => false,
    }
}

//
// EOF
//