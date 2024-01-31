use std::time::Duration;

use tokio::time::sleep;
use warpforge_terminal::Logger;
use warpforge_terminal::Result;

#[tokio::main]
async fn main() -> Result<()> {
	let logger = Logger::new_server(8050).await.unwrap();

	const TASKS: u64 = 10;
	let modules = ["module-a", "module-b", "module-c", "module-d", "module-e"];

	logger.set_upper_max(modules.len() as u64).await?;
	logger.set_lower_max(TASKS).await?;
	for (m, &module) in modules.iter().enumerate() {
		logger.set_upper(module).await?;
		logger.set_upper_position(m as u64 + 1).await?;
		logger
			.log(format!("Start work on module '{module}'...\n"))
			.await?;

		for task in 0..TASKS {
			logger.set_lower(format!("task{}", task)).await?;
			logger.set_lower_position(task + 1).await?;
			sleep(Duration::from_secs(1)).await;
			logger.log(format!("Finished task 'task{task}'\n")).await?;
		}

		logger.log(format!("Finished module '{module}'\n")).await?;
	}

	sleep(Duration::from_secs(3)).await;

	Ok(())
}
