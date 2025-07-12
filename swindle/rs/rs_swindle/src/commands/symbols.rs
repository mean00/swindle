/*
 *
 *      Generic get symbol engine from gdb
 *      we call the different symbols group one per one
 *      so each of them can fill their info block
 *
 */

// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::commands::mon_rtt as rttsym;
use crate::freertos::freertos_symbols as fosym;
use crate::rtt_consts;
crate::setup_log!(false);
use crate::bmplog;
crate::gdb_print_init!();
/**
 * This structure describes a symbol client
 */
struct list_of_symbols {
    /// list of symbols to search for as &[&str]
    symbols: &'static [&'static str],
    /// callback to process one info, empty string if not avail
    processing: fn(&str, &str) -> bool,
    /// callback to clear previously loaded info
    clear: fn() -> bool,
}
const NB_OF_SYMBOL_TABLE: usize = 2;
const symbols_to_collect: [list_of_symbols; NB_OF_SYMBOL_TABLE] = [
    list_of_symbols {
        symbols: &fosym::FreeRTOSSymbolName,
        processing: fosym::freertos_processing,
        clear: fosym::freertos_clear_symbols,
    },
    list_of_symbols {
        symbols: &rtt_consts::RTTSymbolName,
        processing: rttsym::rtt_processing,
        clear: rttsym::rtt_clear_symbols,
    },
];
/**
 * This is used to do bookkeeping of the symbol parser
 */
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
    let mut e = crate::encoder::encoder::new();
    e.begin();
    e.add("qSymbol:");
    e.hex_and_add(name);
    e.end();
    true
}
/**
 *  This calls all the clients to clear up previously loaded symbols
 */
#[unsafe(no_mangle)]
pub fn reset_symbols() {
    bmplog!("Clearing symbols\n");
    let indeces: &mut parser_index = get_index();
    indeces.table_index = 0;
    indeces.line_index = 0;
    for ref i in symbols_to_collect {
        (i.clear)();
    }
}
/**
 *  This is the processing function of the gdb qSymbol call
 *
 */
#[unsafe(no_mangle)]
pub fn q_symbols(args: &[&str]) -> bool {
    let indeces = get_index();
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
    let value: &str = if args.is_empty() { "" } else { args[0] };
    bmplog!("Key {} value {}\n", key, value);
    (symbols_to_collect[indeces.table_index].processing)(key, value);
    indeces.line_index += 1;
    if update_indeces(indeces) {
        ask_for_next_symbol(symbols_to_collect[indeces.table_index].symbols[indeces.line_index]);
    }
    true
}
// EOF
