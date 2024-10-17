//! Sends log messages and progress updates over tcp streams.
//!
//! # Simple Example
//!
//! ```
//! use warpforge_terminal::{logln, Logger};
//!
//! # fn main() {
//! Logger::set_global(Logger::new_local()).unwrap();
//!
//! logln!("Hello, World!");
//! logln!("format {}", 42);
//! # }
//! ```
//!
//! # More Examples
//!
//! More examples can be found in the `examples` folder.
//! Run them using: `cargo run --example <name>`
//!
//! The `client` example requires one of the `server_*` examples to be run first.

mod client;
mod errors;
mod macros;
mod render;
mod server;

use std::io;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;

use crossbeam_channel::Sender;
use errors::GlobalLoggerAlreadyDefined;
use rand::Rng;
use serde::{Deserialize, Serialize};
use server::Server;

pub use crate::client::render_remote_logs;
pub use crate::errors::Error;
pub use crate::errors::Result;
pub use crate::macros::log_global;
use crate::render::TerminalRenderer;

static LOGGER: OnceLock<Logger> = OnceLock::new();

#[derive(Clone)]
pub struct Logger {
	channel: Sender<Message>,
}

impl Logger {
	pub fn new_local() -> Self {
		let (sender, receiver) = crossbeam_channel::bounded(32);
		TerminalRenderer::start(receiver);
		Self { channel: sender }
	}

	pub fn new_server(port: u16) -> std::result::Result<Self, io::Error> {
		let (sender, receiver) = crossbeam_channel::bounded(32);
		Server::new(receiver).start(port)?;
		Ok(Self { channel: sender })
	}

	pub fn set_global(logger: Logger) -> std::result::Result<(), GlobalLoggerAlreadyDefined> {
		LOGGER.set(logger).map_err(|_| GlobalLoggerAlreadyDefined)
	}

	pub fn get_global() -> Option<&'static Logger> {
		LOGGER.get()
	}

	pub fn log(&self, level: Level, message: impl Into<String>) -> Result<()> {
		send_serializable(&self.channel, Serializable::Log(level, message.into()))
	}

	pub fn trace(&self, message: impl Into<String>) -> Result<()> {
		if cfg!(debug_assertions) {
			self.log(Level::Trace, message)
		} else {
			Ok(())
		}
	}

	pub fn debug(&self, message: impl Into<String>) -> Result<()> {
		if cfg!(debug_assertions) {
			self.log(Level::Debug, message)
		} else {
			Ok(())
		}
	}

	pub fn info(&self, message: impl Into<String>) -> Result<()> {
		self.log(Level::Info, message)
	}

	pub fn warn(&self, message: impl Into<String>) -> Result<()> {
		self.log(Level::Warn, message)
	}

	pub fn error(&self, message: impl Into<String>) -> Result<()> {
		self.log(Level::Error, message)
	}

	pub fn create_bar(&self, max: u64, text: impl Into<String>) -> Bar {
		Bar::with_logger(max, text, self)
	}

	pub fn close(&self) -> Result<()> {
		let (sender, receiver) = mpsc::channel();
		send(&self.channel, Message::CloseLocalRenderer(sender))?;
		// Wait for notification from receiver but
		// wait no longer than the defined max time.
		let _ = receiver.recv_timeout(Duration::from_millis(100));
		Ok(())
	}
}

#[derive(Default, Clone)]
pub struct Bar {
	id: BarId,
	channel: Option<Sender<Message>>,
}

impl Drop for Bar {
	fn drop(&mut self) {
		self.send(Serializable::RemoveBar(self.id));
	}
}

impl Bar {
	/// Create new progress bar using the global logger.
	///
	/// If no global logger exists, a "ghost" progress bar is created.
	/// This progress bar can be used without causing panics, but will not
	/// display anything on screen.
	pub fn new(max: u64, text: impl Into<String>) -> Self {
		if let Some(logger) = Logger::get_global() {
			Bar::with_logger(max, text, logger)
		} else {
			Default::default()
		}
	}

	/// Create new progress bar using the given logger.
	pub fn with_logger(max: u64, text: impl Into<String>, logger: &Logger) -> Self {
		let bar = Self {
			id: BarId::new(),
			channel: Some(logger.channel.clone()),
		};
		bar.send(Serializable::CreateBar { id: bar.id, max });
		bar.set_text(text);
		bar
	}

	pub fn set(&self, position: u64, text: impl Into<String>) {
		self.set_position(position);
		self.set_text(text);
	}

	pub fn set_text(&self, text: impl Into<String>) {
		self.send(Serializable::SetBarText(self.id, text.into()));
	}

	pub fn set_position(&self, position: u64) {
		self.send(Serializable::SetBarPosition(self.id, position));
	}

	pub fn set_max(&self, max: u64) {
		self.send(Serializable::SetBarMax(self.id, max));
	}

	#[inline]
	fn send(&self, message: Serializable) {
		let Some(channel) = &self.channel else {
			return;
		};

		let result = send_serializable(channel, message);
		match result {
			Ok(_) => {}
			Err(Error::ChannelInternal { .. }) => {}
		}
	}
}

#[derive(Debug)]
pub enum Message {
	Serializable(Serializable),

	/// Closes the local renderer, if it exists which sends a notification over the given
	/// oneshot channel once all messages are rendered to the terminal.
	/// If no local renderer is attached, the oneshot channel is droped.
	CloseLocalRenderer(mpsc::Sender<()>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Serializable {
	Log(Level, String),
	CreateBar { id: BarId, max: u64 },
	RemoveBar(BarId),
	SetBarText(BarId, String),
	SetBarPosition(BarId, u64),
	SetBarMax(BarId, u64),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Level {
	Trace,
	Debug,
	Info,
	Warn,
	Error,
}

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BarId(u64);

impl BarId {
	pub fn new() -> Self {
		Self(rand::thread_rng().gen())
	}
}

impl PartialEq for Message {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Serializable(left), Self::Serializable(right)) => left == right,
			(Self::CloseLocalRenderer(_), Self::CloseLocalRenderer(_)) => true,
			_ => false,
		}
	}
}

fn send_serializable(channel: &Sender<Message>, message: Serializable) -> Result<()> {
	send(channel, Message::Serializable(message))
}

fn send(channel: &Sender<Message>, message: Message) -> Result<()> {
	channel
		.send(message)
		.map_err(|e| Error::ChannelInternal { input: e.0 })
}
