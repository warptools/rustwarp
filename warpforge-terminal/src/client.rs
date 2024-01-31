use std::io;

use tokio::{
	io::{AsyncRead, AsyncReadExt},
	net::{TcpStream, ToSocketAddrs},
	sync::mpsc::{self, Sender},
};
use tokio_util::sync::CancellationToken;

use crate::{render::TerminalRenderer, Message};

const BUFFER_SIZE: usize = 4096;
const BUFFER_MIN_READ_SIZE: usize = 1024;
const BUFFER_MAX_SIZE: usize = 65536;

pub async fn render_remote_logs(
	address: impl ToSocketAddrs,
) -> Result<CancellationToken, io::Error> {
	let connection = TcpStream::connect(address).await?;
	let (sender, receiver) = mpsc::channel(32);
	TerminalRenderer::start(receiver);
	Ok(start_client(sender, connection))
}

fn start_client<R>(channel: Sender<Message>, mut reader: R) -> CancellationToken
where
	R: AsyncRead + Unpin + Send + 'static,
{
	let token = CancellationToken::new();
	let cloned_token = token.clone();

	tokio::spawn(async move {
		let mut buffer = Vec::with_capacity(BUFFER_SIZE);
		let mut size = 0;
		loop {
			if buffer.capacity() - size < BUFFER_MIN_READ_SIZE
				&& buffer.capacity() + BUFFER_SIZE <= BUFFER_MAX_SIZE
			{
				buffer.reserve(BUFFER_SIZE)
			}

			if size >= buffer.capacity() {
				eprintln!("received message exceeds memory limit");
				return;
			}

			let result = tokio::select! {
				result = reader.read_buf(&mut buffer) => result,
				_ = token.cancelled() => {
					return; // graceful shutdown
				}
			};

			let n = match result {
				Ok(0) => {
					eprintln!("server closed connection");
					return;
				}
				Ok(n) => n,
				Err(error) => {
					eprintln!("failed to receive remote updates: {error}");
					return;
				}
			};

			let mut start = 0;
			for i in size..(size + n) {
				if buffer[i] == b'\0' {
					let message = match serde_json::from_slice(&buffer[start..i]) {
						Ok(message) => message,
						Err(error) => {
							eprintln!("failed to parse received message: {error}");
							return;
						}
					};

					start = i + 1;
					let result = channel.send(message).await;
					if result.is_err() {
						eprintln!("terminal renderer closed channel unexpectedly");
						eprintln!("use the cancellation token to shutdown the client gracefully");
						return;
					}
				}
			}

			size += n;
			if start > 0 {
				buffer.drain(..start);
				size -= start;
			}
		}
	});

	cloned_token
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use tokio_test::io::Builder;

	use super::*;
	use crate::Message;

	trait BuilderExtension {
		fn read_message(&mut self, message: &Message) -> &mut Self;
	}

	impl BuilderExtension for tokio_test::io::Builder {
		fn read_message(&mut self, message: &Message) -> &mut Self {
			let mut bytes = Vec::new();
			serde_json::to_writer(&mut bytes, message).unwrap();
			bytes.push(0);
			self.read(&bytes)
		}
	}

	#[tokio::test]
	async fn simple_message() {
		let message = Message::Log("hi".to_string());
		let reader = Builder::new().read_message(&message).build();
		let (sender, mut receiver) = mpsc::channel(1);
		start_client(sender, reader);

		assert_eq!(Some(message), receiver.recv().await);
	}

	#[tokio::test]
	async fn multiple_messages() {
		let messages = [
			Message::Log("first".to_string()),
			Message::SetUpperMax(5),
			Message::SetUpperPosition(2),
			Message::Log("last".to_string()),
		];

		let mut builder = Builder::new();
		for message in &messages {
			builder.read_message(message);
		}
		let reader = builder.build();

		let (sender, mut receiver) = mpsc::channel(32);
		start_client(sender, reader);

		for message in messages {
			assert_eq!(Some(message), receiver.recv().await);
		}
	}

	#[tokio::test]
	async fn exceed_memory_limit() {
		let message = Message::Log("x".repeat(BUFFER_MAX_SIZE));
		let reader = Builder::new().read_message(&message).build();
		let (sender, mut receiver) = mpsc::channel(1);
		start_client(sender, reader);

		assert_eq!(None, receiver.recv().await);
	}

	#[tokio::test]
	async fn server_closed_connection() {
		let message = Message::Log("hi".to_string());
		let reader = Builder::new().read_message(&message).build();
		let (sender, mut receiver) = mpsc::channel(1);
		start_client(sender, reader);

		assert_eq!(Some(message), receiver.recv().await);
		assert_eq!(None, receiver.recv().await);
	}

	#[tokio::test]
	async fn invalid_message() {
		let reader = Builder::new().read(b"not json\0").build();
		let (sender, mut receiver) = mpsc::channel(1);
		start_client(sender, reader);

		assert_eq!(None, receiver.recv().await);
	}

	#[tokio::test]
	async fn graceful_shutdown() {
		let message = Message::Log("hi".to_string());
		let reader = Builder::new()
			.wait(Duration::from_secs(5))
			.read_message(&message)
			.build();
		let (sender, mut receiver) = mpsc::channel(1);
		let token = start_client(sender, reader);
		token.cancel();

		// If the graceful shutdown does not work, the client will wait for the
		// given duration and then receive `Some(message)` (instead of None).
		assert_eq!(None, receiver.recv().await);
	}
}
