use std::time::Duration;

use tokio::time::sleep;
use warpforge_terminal::Logger;

#[tokio::main]
async fn main() {
	let logger = Logger::new_local();
	logger.log("Hello, World!\n").await.unwrap();
	sleep(Duration::from_secs(1)).await;
}
