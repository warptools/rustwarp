use std::thread::sleep;
use std::time::Duration;

use warpforge_terminal::logln;
use warpforge_terminal::Logger;
use warpforge_terminal::Result;

fn main() {
	// Setup global logger.
	Logger::set_global(Logger::new_local()).unwrap();

	// Simulate tasks that use the logger.
	do_tasks().unwrap();

	let _ = Logger::get_global().unwrap().close();
}

fn do_tasks() -> Result<()> {
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

	Ok(())
}
