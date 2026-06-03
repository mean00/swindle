// logger_native.rs — Native (embedded) logger implementation
// Provides logger! and logger_init! macros for embedded targets.
// Included when #[cfg(not(feature = "hosted"))]

#[macro_export]
macro_rules! logger {
    ($x:expr) => {
        $crate::bmplogger::G::print_str($x);
    };

    ($x:expr, $($y:expr),+) => {
        $crate::gdb_print!($x, $($y),+);
    };
}

#[macro_export]
macro_rules! logger_init {
    () => {};
}