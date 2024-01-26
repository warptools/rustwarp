use std::{io, net::Ipv6Addr};

use tokio::{
	io::AsyncWriteExt,
	net::{TcpListener, TcpStream},
	sync::mpsc::Receiver,
};

use crate::Message;

pub(crate) struct Server {
	channel: Receiver<Message>,
}

impl Server {
	pub(crate) fn new(channel: Receiver<Message>) -> Self {
		Self { channel }
	}

	pub async fn start(self, port: u16) -> Result<(), io::Error> {
		let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, port)).await?;

		tokio::spawn(async move {
			let Self { mut channel } = self;
			let mut message_cache = Vec::new();
			let mut clients = Vec::new();

			loop {
				tokio::select! {
					message = channel.recv() => {
						let Some(message) = message else {
							break; // Stop server after all Logger instances got destroyed.
						};

						let bytes = serialize_message(&message);
						for i in (0..clients.len()).rev() {
							let success = send(&mut clients[i], &bytes[..]).await;
							if !success {
								clients.swap_remove(i);
							}
						}

						// FIXME: Currently grows indefinetely.
						// We will have to decide on a trade-off on how many messages we want to cache
						// or on how many clients we want to wait.
						message_cache.push(message);
					}
					client = listener.accept() => {
						let mut client = match client {
							Ok((stream, _)) => stream,
							Err(error) => {
								eprintln!("[log server] error: failed to accept client: {error}");
								continue;
							}
						};

						let mut success = true;
						for message in &message_cache {
							success = success && send(&mut client, &serialize_message(message)[..]).await;
						}
						if success {
							clients.push(client);
						}
					}
				}
			}
		});

		Ok(())
	}
}

fn serialize_message(message: &Message) -> Vec<u8> {
	let mut bytes =
		serde_json::to_vec(message).expect("[log server] error: failed to serialize message");
	bytes.push(0);
	bytes
}

async fn send(client: &mut TcpStream, bytes: &[u8]) -> bool {
	let result = client.write_all(bytes).await;
	if result.is_err() {
		let _ignore = client.shutdown().await;
		false
	} else {
		true
	}
}
