// logger_native.rs — Native (embedded) logger implementation
// Provides logger! and logger_init! macros for embedded targets.
// Included when #[cfg(not(feature = "hosted"))]

#[macro_export]
macro_rules! logger {
    ($x:expr) => {{
        use ::ufmt::uwrite;
        uwrite!(&mut rust_esprit::logger::LoggerWriter, "{}", $x).unwrap()
    }};

    ($x:expr, $($y:expr),+) => {{
        use ::ufmt::uwrite;
        uwrite!(&mut rust_esprit::logger::LoggerWriter, $x, $($y),+).unwrap()
    }};
}

#[macro_export]
macro_rules! logger_init {
    () => {};
}
