use std::{
	io::{self, ErrorKind, Write},
	net::{Ipv6Addr, TcpListener, TcpStream},
	thread,
	time::Duration,
};

use crossbeam_channel::Receiver;

use crate::{Message, Serializable};

pub(crate) struct Server {
	channel: Receiver<Message>,
}

impl Server {
	pub(crate) fn new(channel: Receiver<Message>) -> Self {
		Self { channel }
	}

	pub fn start(self, port: u16) -> Result<(), io::Error> {
		let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, port))?;
		listener.set_nonblocking(true)?;

		thread::spawn(move || {
			let Self { channel } = self;
			let mut message_cache = Vec::new();
			let mut clients = Vec::new();

			loop {
				match listener.accept() {
					Ok((mut client, _)) => {
						let mut success = true;
						for message in &message_cache {
							success = success && send(&mut client, &serialize_message(message)[..]);
						}
						if success {
							clients.push(client);
						}
					}
					Err(error) if error.kind() == ErrorKind::WouldBlock => {}
					Err(error) if error.kind() == ErrorKind::TimedOut => {}
					Err(error) => {
						eprintln!("[log server] error: failed to accept client: {error}");
						continue;
					}
				}

				let message = match channel.recv_timeout(Duration::from_millis(100)) {
					Ok(Message::Serializable(message)) => message,
					Ok(_) => continue, // Cannot send non-serializable messages.
					Err(error) if error.is_timeout() => continue,
					_ => break, // Stop server after all Logger instances got destroyed.
				};

				let bytes = serialize_message(&message);
				for i in (0..clients.len()).rev() {
					let success = send(&mut clients[i], &bytes[..]);
					if !success {
						clients.swap_remove(i);
					}
				}

				// FIXME: Currently grows indefinetely.
				// We will have to decide on a trade-off on how many messages we want to cache
				// or on how many clients we want to wait.
				message_cache.push(message);
			}
		});

		Ok(())
	}
}

fn serialize_message(message: &Serializable) -> Vec<u8> {
	let mut bytes =
		serde_json::to_vec(message).expect("[log server] error: failed to serialize message");
	bytes.push(0);
	bytes
}

fn send(client: &mut TcpStream, bytes: &[u8]) -> bool {
	let result = client.write_all(bytes);
	if result.is_err() {
		let _ignore = client.shutdown(std::net::Shutdown::Both);
		false
	} else {
		true
	}
}
