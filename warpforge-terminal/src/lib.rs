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
	Log(String),
	SetUpper(String),
	SetLower(String),
	SetUpperPosition(u64),
	SetLowerPosition(u64),
	SetUpperMax(u64),
	SetLowerMax(u64),
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

	pub fn log(&self, message: impl Into<String>) -> Result<()> {
		self.send_serializable(Serializable::Log(message.into()))
	}

	pub fn set_upper(&self, name: impl Into<String>) -> Result<()> {
		self.send_serializable(Serializable::SetUpper(name.into()))
	}

	pub fn set_lower(&self, name: impl Into<String>) -> Result<()> {
		self.send_serializable(Serializable::SetLower(name.into()))
	}

	pub fn set_upper_position(&self, position: u64) -> Result<()> {
		self.send_serializable(Serializable::SetUpperPosition(position))
	}

	pub fn set_lower_position(&self, position: u64) -> Result<()> {
		self.send_serializable(Serializable::SetLowerPosition(position))
	}

	pub fn set_upper_max(&self, max: u64) -> Result<()> {
		self.send_serializable(Serializable::SetUpperMax(max))
	}

	pub fn set_lower_max(&self, max: u64) -> Result<()> {
		self.send_serializable(Serializable::SetLowerMax(max))
	}

	pub fn close(&self) -> Result<()> {
		let (sender, receiver) = mpsc::channel();
		self.send(Message::CloseLocalRenderer(sender))?;
		// Wait for notification from receiver but
		// wait no longer than the defined max time.
		let _ = receiver.recv_timeout(Duration::from_millis(100));
		Ok(())
	}

	fn send_serializable(&self, message: Serializable) -> Result<()> {
		self.send(Message::Serializable(message))
	}

	fn send(&self, message: Message) -> Result<()> {
		self.channel
			.send(message)
			.map_err(|e| Error::ChannelInternal { input: e.0 })
	}
}

pub fn set_upper(name: impl Into<String>) {
	if let Some(logger) = Logger::get_global() {
		logger.set_upper(name).unwrap();
	}
}

pub fn set_lower(name: impl Into<String>) {
	if let Some(logger) = Logger::get_global() {
		logger.set_lower(name).unwrap();
	}
}

pub fn set_upper_position(position: u64) {
	if let Some(logger) = Logger::get_global() {
		logger.set_upper_position(position).unwrap();
	}
}

pub fn set_lower_position(position: u64) {
	if let Some(logger) = Logger::get_global() {
		logger.set_lower_position(position).unwrap();
	}
}

pub fn set_upper_max(max: u64) {
	if let Some(logger) = Logger::get_global() {
		logger.set_upper_max(max).unwrap();
	}
}

pub fn set_lower_max(max: u64) {
	if let Some(logger) = Logger::get_global() {
		logger.set_lower_max(max).unwrap();
	}
}
