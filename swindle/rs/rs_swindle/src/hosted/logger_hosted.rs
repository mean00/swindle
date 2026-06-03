// logger_hosted.rs — Hosted (desktop) logger implementation
// Provides logger! and logger_init! macros for hosted builds.
// Included when #[cfg(feature = "hosted")]

#[macro_export]
macro_rules! logger {
    ($x:expr) => {
        print!("{}", $x)
    };

    ($x:expr, $($y:expr),+) => {
        print!($x, $($y),+)
    };
}

#[macro_export]
macro_rules! logger_init {
    () => {
        extern crate std;
    };
}