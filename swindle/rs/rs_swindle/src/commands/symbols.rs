//! GDB `qSymbol` symbol lookup engine.
//!
//! Implements the GDB remote protocol `qSymbol` packet for looking up
//! symbol values from the target's ELF symbol table.
//!
//! The engine iterates through multiple symbol groups (FreeRTOS, RTT, etc.)
//! and asks GDB for each symbol's value. Each group provides:
//!
//! - A list of symbol names to look up
//! - A processing callback to handle the returned value
//! - A clear callback to reset previously loaded state
//!
//! ## Symbol groups
//!
//! | Group | Symbols | Purpose |
//! |-------|---------|---------|
//! | FreeRTOS | `FreeRTOSSymbolName` | Locate FreeRTOS kernel structures |
//! | RTT | `_SEGGER_RTT` | Locate SEGGER RTT control block |
//!
//! ## References
//!
//! - <https://sourceware.org/gdb/onlinedocs/gdb/Packets.html>
//! - <https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html>


use crate::commands::mon_rtt as rttsym;
use crate::freertos::freertos_symbols as fosym;
use crate::rtt_consts;
setup_log!(false);
//use crate::bmplog;
crate::gdb_print_init!();
/// Describes a symbol client: a group of symbols to look up and their
/// processing/clear callbacks.
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
/// Bookkeeping state for the symbol parser iteration.
use core::cell::UnsafeCell;

// Safe: only accessed from single-threaded embedded context
struct SyncCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncCell<T> {}
impl<T> SyncCell<T> {
    const fn new(val: T) -> Self { SyncCell(UnsafeCell::new(val)) }
    fn get(&self) -> T where T: Copy { unsafe { *self.0.get() } }
    fn set(&self, val: T) where T: Copy { unsafe { *self.0.get() = val; } }
}

#[derive(Clone, Copy)]
struct parser_index {
    table_index: usize,
    line_index: usize,
}
//
static symbol_indeces: SyncCell<parser_index> = SyncCell::new(parser_index {
    table_index: 0,
    line_index: 0,
});
/*
 *
 */
fn get_index() -> parser_index {
    symbol_indeces.get()
}
fn set_index(val: parser_index) {
    symbol_indeces.set(val);
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
/// Clear all previously loaded symbol values across all groups.
#[unsafe(no_mangle)]
pub fn reset_symbols() {
    bmplog!("Clearing symbols\n");
    set_index(parser_index {
        table_index: 0,
        line_index: 0,
    });
    for ref i in symbols_to_collect {
        (i.clear)();
    }
}
/// Process a `qSymbol` response from GDB.
///
/// If the response is empty (both args empty), starts a new symbol
/// lookup cycle. Otherwise, stores the returned value and asks for
/// the next symbol.
#[unsafe(no_mangle)]
pub fn q_symbols(args: &[&str]) -> bool {
    let mut indeces = get_index();
    // empty one = let's start
    if args[0].is_empty() && args[1].is_empty() {
        reset_symbols();
        indeces = get_index();
        ask_for_next_symbol(symbols_to_collect[indeces.table_index].symbols[indeces.line_index]);
        return true;
    }
    if indeces.table_index >= NB_OF_SYMBOL_TABLE {
        // all done
        return false;
    }
    let key = symbols_to_collect[indeces.table_index].symbols[indeces.line_index];
    let value: &str = if args.is_empty() { "" } else { args[0] };
    bmplog!("Key {} value {}\n", key, value);
    (symbols_to_collect[indeces.table_index].processing)(key, value);
    indeces.line_index += 1;
    if update_indeces(&mut indeces) {
        set_index(indeces);
        ask_for_next_symbol(symbols_to_collect[indeces.table_index].symbols[indeces.line_index]);
        true
    } else {
        set_index(indeces);
        false
    }
}
// EOF
