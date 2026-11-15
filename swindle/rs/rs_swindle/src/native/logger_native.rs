// logger_native.rs — Native (embedded) logger implementation
// Provides logger! and logger_init! macros for embedded targets.
// Included when #[cfg(not(feature = "hosted"))]

#[macro_export]
macro_rules! logger {
    ($x:literal) => {{
        use ::ufmt::uWrite;
        let _ = rust_esprit::LoggerWriter.write_str($x);
    }};

    ($x:expr) => {{
        use ::ufmt::uwrite;
        let _ = uwrite!(&mut rust_esprit::LoggerWriter, "{}", $x);
    }};

    ($x:expr, $($y:expr),+) => {{
        use ::ufmt::uwrite;
        let _ = uwrite!(&mut rust_esprit::LoggerWriter, $x, $($y),+);
    }};
}

#[macro_export]
macro_rules! logger_init {
    () => {};
}
