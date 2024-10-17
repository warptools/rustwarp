use warpforge_terminal::Logger;

fn main() {
	let logger = Logger::new_local();
	logger.info("Hello, World!\n").unwrap();
	let _ = logger.close();
}
