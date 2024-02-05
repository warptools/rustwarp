use warpforge_terminal::{log, logln, Logger};

#[tokio::main]
async fn main() {
	Logger::set_global(Logger::new_local()).unwrap();
	log!("{}, {}!\n", "hello", "world");
	logln!("{}, {}", 42, 1337);
}
