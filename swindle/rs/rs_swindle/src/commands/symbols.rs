/*
 *
 *      Generic get symbol engine from gdb
 *      we call the different symbols group one per one
 *
 */

// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::encoder::encoder;

use crate::freertos::freertos_symbols as fosym;
use crate::parsing_util;
crate::setup_log!(true);
use crate::{bmplog, bmpwarning};
crate::gdb_print_init!();
use crate::gdb_print;

struct list_of_symbols {
    symbols: &'static [&'static str],
    processing: fn(&str, &str) -> bool,
}
const NB_OF_SYMBOL_TABLE: usize = 2;
const symbols_to_collect: [list_of_symbols; NB_OF_SYMBOL_TABLE] = [
    list_of_symbols {
        symbols: &fosym::FreeRTOSSymbolName,
        processing: fosym::freertos_processing,
    },
    list_of_symbols {
        symbols: &fosym::FreeRTOSSymbolName,
        processing: fosym::freertos_processing,
    },
];

struct parser_index {
    table_index: usize,
    line_index: usize,
}
//
static mut symbol_indeces: parser_index = parser_index {
    table_index: 0,
    line_index: 0,
};
/*
 *
 */
fn get_index() -> &'static mut parser_index {
    unsafe { &mut symbol_indeces }
}
/*
 *
 */
pub fn reset_symbols() {
    let indeces: &mut parser_index = get_index();
    indeces.table_index = 0;
    indeces.line_index = 0;
}
/*
 *
 */
fn update_indeces(indeces: &mut parser_index) -> bool {
    if indeces.table_index >= NB_OF_SYMBOL_TABLE {
        // all done
        return false;
    }
    // next table
    if indeces.line_index >= symbols_to_collect[indeces.table_index].symbols.len() {
        indeces.table_index += 1;
        indeces.line_index = 0;
        if indeces.table_index >= NB_OF_SYMBOL_TABLE {
            return false;
        }
    }
    true
}
/*
 *
 *
 */
fn ask_for_next_symbol(name: &str) -> bool {
    let mut e = encoder::new();
    e.begin();
    e.add("qSymbol:");
    e.hex_and_add(name);
    e.end();
    true
}

/*
 *
 *
 */
#[unsafe(no_mangle)]
pub fn q_symbols(args: &[&str]) -> bool {
    let mut indeces = get_index();
    // empty one = let's start
    if args[0].is_empty() && args[1].is_empty() {
        reset_symbols();
        ask_for_next_symbol(symbols_to_collect[indeces.table_index].symbols[indeces.line_index]);
        return true;
    }
    if indeces.table_index >= NB_OF_SYMBOL_TABLE {
        // all done
        return true;
    }
    let key = symbols_to_collect[indeces.table_index].symbols[indeces.line_index];
    let value: &str;
    if args.is_empty() {
        value = "";
    } else {
        value = args[0];
    }
    gdb_print!("Key {}", key);
    gdb_print!("value {}", value);
    (symbols_to_collect[indeces.table_index].processing)(key, value);
    indeces.line_index += 1;
    if update_indeces(&mut indeces) {
        ask_for_next_symbol(symbols_to_collect[indeces.table_index].symbols[indeces.line_index]);
    }
    return true;
}
