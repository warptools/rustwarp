use std::time::Duration;

use tokio::time::sleep;
use warpforge_terminal::logln;
use warpforge_terminal::Logger;
use warpforge_terminal::Result;

#[tokio::main]
async fn main() {
	// Setup global logger.
	Logger::set_global(Logger::new_local()).unwrap();

	// Simulate tasks that use the logger.
	do_tasks().await.unwrap();

	sleep(Duration::from_secs(3)).await;
}

async fn do_tasks() -> Result<()> {
	let logger = Logger::get_global().unwrap();

	const TASKS: u64 = 10;
	let modules = ["module-a", "module-b", "module-c", "module-d", "module-e"];

	logger.set_upper_max(modules.len() as u64).await?;
	logger.set_lower_max(TASKS).await?;
	for (m, &module) in modules.iter().enumerate() {
		logger.set_upper(module).await?;
		logln!("Start work on module '{module}'...");

		for task in 0..TASKS {
			logger.set_lower(format!("task{}", task)).await?;
			logger.set_lower_position(task).await?;
			sleep(Duration::from_secs(1)).await;
			logln!("Finished task 'task{task}'");
		}

		logln!("Finished module '{module}'");
		logger.set_lower_position(TASKS).await?;
		logger.set_upper_position(m as u64 + 1).await?;
	}

	Ok(())
}
