mod errors;
mod render;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Sender};

pub use crate::errors::Error;
use crate::errors::Result;
use crate::render::TerminalRenderer;

#[derive(Clone)]
pub struct Logger {
	channel: Sender<Message>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
