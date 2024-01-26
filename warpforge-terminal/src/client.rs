use std::io;

use tokio::{
	io::AsyncReadExt,
	net::{TcpStream, ToSocketAddrs},
	sync::mpsc,
};

use crate::render::TerminalRenderer;

const BUFFER_SIZE: usize = 4096;
const BUFFER_MIN_READ_SIZE: usize = 1024;
const BUFFER_MAX_SIZE: usize = 65536;

pub async fn render_remote_logs(address: impl ToSocketAddrs) -> Result<(), io::Error> {
	let mut connection = TcpStream::connect(address).await?;
	let (sender, receiver) = mpsc::channel(32);
	TerminalRenderer::start(receiver);

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

			let n = match connection.read_buf(&mut buffer).await {
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
					let result = sender.send(message).await;
					if result.is_err() {
						eprintln!("terminal renderer closed channel unexpectedly");
						// TODO: explain graceful shutdown of client
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

	Ok(())
}
