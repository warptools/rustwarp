use std::thread::sleep;
use std::time::Duration;

use warpforge_terminal::Bar;
use warpforge_terminal::Logger;
use warpforge_terminal::Result;

fn main() -> Result<()> {
	let logger = Logger::new_server(8050).unwrap();

	const TASKS: u64 = 10;
	let modules = ["module-a", "module-b", "module-c", "module-d", "module-e"];

	let upper = Bar::new(modules.len() as u64, "");
	let lower = Bar::new(TASKS, "");
	for (m, &module) in modules.iter().enumerate() {
		upper.set_text(module);
		logger.log(format!("Start work on module '{module}'...\n"))?;

		for task in 0..TASKS {
			lower.set(task, format!("task{}", task));
			sleep(Duration::from_millis(100));
			logger.log(format!("Finished task 'task{task}'\n"))?;
		}

		logger.log(format!("Finished module '{module}'\n"))?;
		lower.set_position(TASKS);
		upper.set_position(m as u64 + 1);
	}

	sleep(Duration::from_secs(3));

	Ok(())
}
