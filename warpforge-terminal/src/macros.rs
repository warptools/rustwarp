use crate::{Error, Level, Logger, Message, Serializable};

/// Helper function for [`log!`] and [`logln`] macros.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
pub fn log_global(level: Level, message: impl Into<String>) {
	if let Some(logger) = Logger::get_global() {
		match logger.log(level, message.into()) {
			Ok(_) => {}
			Err(Error::ChannelInternal {
				input: Message::Serializable(Serializable::Log(_, message)),
			}) => print!("{}", message),
			Err(e) => panic!("log!() or logln!() failed unexpectedly: {e}"),
		}
	} else {
		print!("{}", message.into());
	}
}

/// Sends a message to the global logger.
///
/// Equivalent to the [`logln!`] macro except that a newline is not sent at
/// the end of the message.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{log, Logger};
///
/// # fn main() {
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
	($($arg:tt)+) => { $crate::log_global($crate::Level::Info, format!($($arg)+)) };
}

/// Sends a message to the global logger, with a newline.
///
/// On all platforms, the newline is the LINE FEED character (`\n`/`U+000A`) alone
/// (no additional CARRIAGE RETURN (`\r`/`U+000D`)).
///
/// This macro uses the same syntax as [`format!`].
///
/// The message is directly written to stdout using [`print!`] if
/// * the global logger was not setup correctly
/// * the global logger was already closed again
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{logln, Logger};
///
/// # fn main() {
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
	() => { $crate::log_global($crate::Level::Info, "\n") };
	// Using two format_args! calls here, to avoid allocation of two String instances.
	// https://github.com/rust-lang/rust/pull/97658#issuecomment-1530505696
	// https://github.com/rust-lang/rust/pull/111060
	($($arg:tt)+) => {{
		let message = std::fmt::format(format_args!("{}\n", format_args!($($arg)+)));
		$crate::log_global($crate::Level::Info, message);
	}};
}

/// Sends a trace message to the global logger.
///
/// Equivalent to the [`logln!`] macro except that the log level is trace
/// and the output is disabled in release mode.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{Logger, trace};
///
/// # fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// trace!("Start trace");
/// trace!("End {}", "trace");
/// # }
/// ```
#[macro_export]
macro_rules! trace {
	($($arg:tt)+) => {
		if cfg!(debug_assertions) {
			let message = std::fmt::format(format_args!("{}\n", format_args!($($arg)+)));
			$crate::log_global($crate::Level::Trace, message);
		}
	};
}

/// Sends a debug message to the global logger.
///
/// Equivalent to the [`logln!`] macro except that the log level is debug
/// and the output is disabled in release mode.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{debug, Logger};
///
/// # fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// debug!("Hello, World!");
/// debug!("format {}", 42);
/// # }
/// ```
#[macro_export]
macro_rules! debug {
	($($arg:tt)+) => {
		if cfg!(debug_assertions) {
			let message = std::fmt::format(format_args!("{}\n", format_args!($($arg)+)));
			$crate::log_global($crate::Level::Debug, message);
		}
	};
}

/// Sends a info message to the global logger.
///
/// Equivalent to the [`logln!`] macro.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{info, Logger};
///
/// # fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// info!("Hello, World!");
/// info!("format {}", 42);
/// # }
/// ```
#[macro_export]
macro_rules! info {
	($($arg:tt)+) => {{
		let message = std::fmt::format(format_args!("{}\n", format_args!($($arg)+)));
		$crate::log_global($crate::Level::Debug, message);
	}};
}

/// Sends a warning message to the global logger.
///
/// Equivalent to the [`logln!`] macro except that the log level is warn.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{Logger, warn};
///
/// # fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// warn!("Hello, World!");
/// warn!("format {}", 42);
/// # }
/// ```
#[macro_export]
macro_rules! warn {
	($($arg:tt)+) => {{
		let message = std::fmt::format(format_args!("{}\n", format_args!($($arg)+)));
		$crate::log_global($crate::Level::Warn, message);
	}};
}

/// Sends a error message to the global logger.
///
/// Equivalent to the [`logln!`] macro except that the log level is error.
///
/// # Panics
///
/// Panics on unexpected errors. Panic was unreachable when writing this comment.
///
/// # Examples
///
/// ```
/// use warpforge_terminal::{error, Logger};
///
/// # fn main() {
/// Logger::set_global(Logger::new_local()).unwrap();
///
/// error!("Hello, World!");
/// error!("format {}", 42);
/// # }
/// ```
#[macro_export]
macro_rules! error {
	($($arg:tt)+) => {{
		let message = std::fmt::format(format_args!("{}\n", format_args!($($arg)+)));
		$crate::log_global($crate::Level::Error, message);
	}};
}
