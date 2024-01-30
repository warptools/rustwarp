pub mod client;
mod errors;
mod render;
mod server;

use std::io;

use serde::{Deserialize, Serialize};
use server::Server;
use tokio::sync::mpsc::{self, Sender};

pub use crate::errors::Error;
use crate::errors::Result;
use crate::render::TerminalRenderer;

#[derive(Clone)]
pub struct Logger {
	channel: Sender<Message>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum Message {
	Log(String),
	SetUpper(String),
	SetLower(String),
	SetUpperPosition(u64),
	SetLowerPosition(u64),
	SetUpperMax(u64),
	SetLowerMax(u64),
}

impl Logger {
	pub fn new_local() -> Self {
		let (sender, receiver) = mpsc::channel(32);
		TerminalRenderer::start(receiver);
		Self { channel: sender }
	}

	pub async fn new_server(port: u16) -> std::result::Result<Self, io::Error> {
		let (sender, receiver) = mpsc::channel(32);
		Server::new(receiver).start(port).await?;
		Ok(Self { channel: sender })
	}

	pub async fn log(&self, message: impl Into<String>) -> Result<()> {
		self.send(Message::Log(message.into())).await
	}

	async fn send(&self, message: Message) -> Result<()> {
		self.channel
			.send(message)
			.await
			.map_err(|_| Error::ChannelInternal)
	}
}
