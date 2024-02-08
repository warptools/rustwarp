use warpforge_terminal::Logger;

#[tokio::main]
async fn main() {
	let logger = Logger::new_local();
	logger.log("Hello, World!\n").await.unwrap();
	let _ = logger.close().await;
}
