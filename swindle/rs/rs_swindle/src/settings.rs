/*
 *  Hold a hashMap so that a given tcb address always gives the same task id
 *   whatever freertos compilation options are.
 *
 */
use arraystring::{ArrayString, typenum::U32};
use core::ptr::addr_of_mut;
use hashbrown::HashMap;
const SW_TOKEN_SIZE: usize = 32;
type token = ArrayString<U32>;
//
crate::setup_log!(false);
use crate::gdb_print;
crate::gdb_print_init!();

/*
 *
 *
 */
struct swindle_settings {
    hash: HashMap<token, u32>,
}
/*
 */
impl swindle_settings {
    /*
     *
     */
    fn new() -> Self {
        swindle_settings {
            hash: HashMap::new(),
        }
    }
    /*
     *
     */
    fn clear(&mut self) {
        self.hash.clear();
    }
}

//--

static mut my_settings: Option<swindle_settings> = None;
/*
 *
 */
fn get_settings() -> &'static mut swindle_settings {
    unsafe {
        if my_settings.is_none() {
            my_settings = Some(swindle_settings::new());
        }
        match &mut *addr_of_mut!(my_settings) {
            Some(x) => x,
            None => panic!("hashap"),
        }
    }
}
/*
 *
 */
pub fn dump() {
    let info = get_settings();
    gdb_print!("Dumping settings \n");
    for (key, value) in &info.hash {
        gdb_print!(
            "Key: {}, Value: {} (dec) - 0x{:x} (hex)\n",
            key.as_str(),
            *value,
            *value as usize
        );
    }
}
/*
 *
 *
 */
fn to_token(key: &str) -> token {
    if key.len() <= SW_TOKEN_SIZE {
        let t: token = token::from(key);
        return t;
    }
    token::from(&key[..SW_TOKEN_SIZE])
}

/*
 *
 */
pub fn set(k: &str, value: u32) {
    let info = get_settings();
    let key: token = to_token(k);
    if info.hash.contains_key(&key) {
        let (_other_key, old_value) = info.hash.get_key_value_mut(&key).unwrap();
        *old_value = value;
    } else {
        info.hash.insert(key, value);
    }
}
/*
 *
 *
 */
pub fn remove(k: &str) -> bool {
    let info = get_settings();
    let key: token = to_token(k);
    if info.hash.contains_key(&key) {
        info.hash.remove(&key);
        return true;
    }
    false
}
/*
 *
 *
 */
pub fn get_or_default(k: &str, def: u32) -> u32 {
    let info = get_settings();
    let key: token = to_token(k);
    match info.hash.get(&key) {
        Some(x) => *x,
        None => def,
    }
}
/*
 *
 *
 */
pub fn init_settings() {
    unsafe {
        my_settings = Some(swindle_settings::new());
    }
}
// EOF
