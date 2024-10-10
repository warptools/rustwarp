use std::{thread::sleep, time::Duration};

use warpforge_terminal::Logger;

fn main() {
	let logger = Logger::new_server(8050).unwrap();
	loop {
		logger.log("Hello, World!\n").unwrap();
		sleep(Duration::from_secs(1));
	}
}
