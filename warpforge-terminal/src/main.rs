use std::{
	io::{stdin, Read},
	net::Ipv4Addr,
};

use warpforge_terminal::render_remote_logs;

fn main() {
	let token = render_remote_logs((Ipv4Addr::LOCALHOST, 8050)).unwrap();
	let _ = stdin().read(&mut [0u8]).unwrap();
	token.cancel();
}
