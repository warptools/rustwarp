use warpforge_terminal::Logger;

fn main() {
	let logger = Logger::new_local();
	logger.log("Hello, World!\n").unwrap();
	let _ = logger.close();
}
