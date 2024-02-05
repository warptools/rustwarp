use crate::Logger;

/// Helper function for [`log!`] and [`logln`] macros.
///
/// # Panics
///
/// Panics if no global logger is defined or
/// the global logger has already been closed.
pub async fn log_global(message: impl Into<String>) {
	let logger = Logger::get_global()
		.expect("log!() and logln!() macros require a global logger (see `Logger::set_global`)");
	logger
		.log(message.into())
		.await
		.expect("log!() or logln!() macro was used, but global logger already terminated");
}

/// Sends a message to the global logger.
///
/// Equivalent to the [`logln!`] macro except that a newline is not sent at
/// the end of the message.
///
/// # Panics
///
/// Panics if no global logger is defined. (See [`crate::Logger::set_global`])
///
/// Writing after the global logger is closed will lead this macro to panic.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{log, Logger};
///
/// # #[tokio::main]
/// # async fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// log!("Hello, ");
/// log!("{}!\n", "World");
/// let foo = "foo";
/// log!("{foo}_bar\n");
/// # }
/// ```
#[macro_export]
macro_rules! log {
	($($arg:tt)+) => { $crate::log_global(format!($($arg)+)).await };
}

/// Sends a message to the global logger, with a newline.
///
/// On all platforms, the newline is the LINE FEED character (`\n`/`U+000A`) alone
/// (no additional CARRIAGE RETURN (`\r`/`U+000D`)).
///
/// This macro uses the same syntax as [`format!`].
///
/// # Panics
///
/// Panics if no global logger is defined. (See [`crate::Logger::set_global`])
///
/// Writing after the global logger is closed will lead this macro to panic.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{logln, Logger};
///
/// # #[tokio::main]
/// # async fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// logln!(); // Logs just a newline
/// logln!("Hello, World!");
/// logln!("format {}", 42);
/// let foo = "foo";
/// logln!("{foo}_bar");
/// # }
/// ```
#[macro_export]
macro_rules! logln {
	() => { $crate::log_global("\n").await };
	// Using two format_args! calls here, to avoid allocation of two String instances.
	// https://github.com/rust-lang/rust/pull/97658#issuecomment-1530505696
	// https://github.com/rust-lang/rust/pull/111060
	($($arg:tt)+) => { $crate::log_global(std::fmt::format(format_args!("{}\n", format_args!($($arg)+)))).await };
}
