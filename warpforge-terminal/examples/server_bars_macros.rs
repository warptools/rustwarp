use std::thread::sleep;
use std::time::Duration;

use warpforge_terminal::logln;
use warpforge_terminal::Logger;
use warpforge_terminal::Result;

fn main() -> Result<()> {
	let logger = Logger::new_server(8050).unwrap();
	Logger::set_global(logger).unwrap();
	let logger = Logger::get_global().unwrap();

	const TASKS: u64 = 10;
	let modules = ["module-a", "module-b", "module-c", "module-d", "module-e"];

	logger.set_upper_max(modules.len() as u64)?;
	logger.set_lower_max(TASKS)?;
	for (m, &module) in modules.iter().enumerate() {
		logger.set_upper(module)?;
		logln!("Start work on module '{module}'...");

		for task in 0..TASKS {
			logger.set_lower(format!("task{}", task))?;
			logger.set_lower_position(task)?;
			sleep(Duration::from_millis(100));
			logln!("Finished task 'task{task}'");
		}

		logln!("Finished module '{module}'");
		logger.set_lower_position(TASKS)?;
		logger.set_upper_position(m as u64 + 1)?;
	}

	sleep(Duration::from_secs(3));

	Ok(())
}
