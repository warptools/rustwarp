use warpforge_terminal::{log, logln, Logger};

fn main() {
	Logger::set_global(Logger::new_local()).unwrap();
	log!("{}, {}!\n", "hello", "world");
	logln!("{}, {}", 42, 1337);

	// Wait for all messages to be printed to stdout.
	let _ = Logger::get_global().unwrap().close();
}
