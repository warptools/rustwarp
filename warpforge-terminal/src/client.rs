use std::{
	io::{self, ErrorKind, Read},
	net::{TcpStream, ToSocketAddrs},
	sync::{Arc, RwLock},
	thread,
	time::Duration,
};

use crossbeam_channel::Sender;

use crate::{render::TerminalRenderer, Message};

const BUFFER_SIZE: usize = 4096;
const BUFFER_MIN_READ_SIZE: usize = 1024;
const BUFFER_MAX_SIZE: usize = 65536;

pub fn render_remote_logs(address: impl ToSocketAddrs) -> Result<CancellationToken, io::Error> {
	let connection = TcpStream::connect(address)?;
	connection.set_read_timeout(Some(Duration::from_millis(100)))?;
	let (sender, receiver) = crossbeam_channel::bounded(32);
	TerminalRenderer::start(receiver);
	Ok(start_client(sender, connection))
}

#[derive(Clone)]
pub struct CancellationToken {
	token: Arc<RwLock<bool>>,
}

impl CancellationToken {
	fn new() -> Self {
		Self {
			token: Arc::new(RwLock::new(false)),
		}
	}

	pub fn cancel(&self) {
		*self.token.write().unwrap() = true;
	}

	pub fn cancelled(&self) -> bool {
		*self.token.read().unwrap()
	}
}

fn start_client<R>(channel: Sender<Message>, mut reader: R) -> CancellationToken
where
	R: Read + Send + 'static,
{
	let token = CancellationToken::new();
	let cloned_token = token.clone();

	thread::spawn(move || {
		let mut buffer = vec![0; BUFFER_SIZE];
		let mut size = 0;
		loop {
			if token.cancelled() {
				return;
			}

			if buffer.len() - size < BUFFER_MIN_READ_SIZE
				&& buffer.len() + BUFFER_SIZE <= BUFFER_MAX_SIZE
			{
				buffer.resize(buffer.len() + BUFFER_SIZE, 0);
			}

			if size >= buffer.len() {
				eprintln!("received message exceeds memory limit");
				return;
			}

			let n = match reader.read(&mut buffer[size..]) {
				Ok(0) => {
					eprintln!("server closed connection");
					return;
				}
				Ok(n) => n,
				Err(error) => {
					if let ErrorKind::TimedOut | ErrorKind::WouldBlock = error.kind() {
						continue;
					} else {
						eprintln!("failed to receive remote updates: {error}");
						return;
					}
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
					let result = channel.send(Message::Serializable(message));
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
	use std::{mem, time::Duration};

	use crossbeam_channel::Receiver;

	use super::*;
	use crate::{BarId, Level, Message, Serializable};

	struct Builder {
		data: Vec<u8>,
	}

	impl Builder {
		fn new() -> Self {
			Self { data: Vec::new() }
		}

		fn read(&mut self, data: &[u8]) -> &mut Self {
			self.data.extend_from_slice(data);
			self
		}

		fn read_message(&mut self, message: &Serializable) -> &mut Self {
			serde_json::to_writer(&mut self.data, message).unwrap();
			self.data.push(0);
			self
		}

		fn build(&mut self) -> &'static [u8] {
			mem::take(&mut self.data).leak()
		}
	}

	fn sender_closed<T>(receiver: &Receiver<T>) -> bool {
		match receiver.recv_timeout(Duration::from_millis(10)) {
			Ok(_) => false,
			Err(error) => dbg!(error).is_disconnected(),
		}
	}

	#[test]
	fn simple_message() {
		let message = Serializable::Log(Level::Info, "hi".to_string());
		let reader = Builder::new().read_message(&message).build();
		let (sender, receiver) = crossbeam_channel::bounded(1);
		start_client(sender, reader);

		assert_eq!(Ok(Message::Serializable(message)), receiver.recv());
	}

	#[test]
	fn multiple_messages() {
		let messages = [
			Serializable::Log(Level::Info, "first".to_string()),
			Serializable::SetBarMax(BarId(7), 5),
			Serializable::SetBarPosition(BarId(7), 2),
			Serializable::Log(Level::Info, "last".to_string()),
		];

		let mut builder = Builder::new();
		for message in &messages {
			builder.read_message(message);
		}
		let reader = builder.build();

		let (sender, receiver) = crossbeam_channel::bounded(32);
		start_client(sender, reader);

		for message in messages {
			assert_eq!(Ok(Message::Serializable(message)), receiver.recv());
		}
	}

	#[test]
	fn exceed_memory_limit() {
		let message = Serializable::Log(Level::Info, "x".repeat(BUFFER_MAX_SIZE));
		let reader = Builder::new().read_message(&message).build();
		let (sender, receiver) = crossbeam_channel::bounded(1);
		start_client(sender, reader);

		assert!(sender_closed(&receiver));
	}

	#[test]
	fn server_closed_connection() {
		let message = Serializable::Log(Level::Info, "hi".to_string());
		let reader = Builder::new().read_message(&message).build();
		let (sender, receiver) = crossbeam_channel::bounded(1);
		start_client(sender, reader);

		assert_eq!(Ok(Message::Serializable(message)), receiver.recv());
		assert!(sender_closed(&receiver));
	}

	#[test]
	fn invalid_message() {
		let reader = Builder::new().read(b"not json\0").build();
		let (sender, receiver) = crossbeam_channel::bounded(1);
		start_client(sender, reader);

		assert!(sender_closed(&receiver));
	}

	#[test]
	fn graceful_shutdown() {
		struct ReadWouldBlock;
		impl Read for ReadWouldBlock {
			fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
				Err(ErrorKind::WouldBlock.into())
			}
		}
		let reader = ReadWouldBlock;

		let (sender, receiver) = crossbeam_channel::bounded(1);
		let token = start_client(sender, reader);
		token.cancel();

		assert!(sender_closed(&receiver));
	}
}
