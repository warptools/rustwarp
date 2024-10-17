use std::thread::sleep;
use std::time::Duration;

use warpforge_terminal::logln;
use warpforge_terminal::trace;
use warpforge_terminal::Bar;
use warpforge_terminal::Logger;

fn main() {
	// Setup global logger.
	Logger::set_global(Logger::new_local()).unwrap();

	// Simulate tasks that use the logger.
	do_tasks();

	let _ = Logger::get_global().unwrap().close();
}

fn do_tasks() {
	trace!("Start do_tasks()");

	const TASKS: u64 = 10;
	let modules = ["module-a", "module-b", "module-c", "module-d", "module-e"];

	let upper = Bar::new(modules.len() as u64, "");
	let lower = Bar::new(TASKS, "");
	for (m, &module) in modules.iter().enumerate() {
		upper.set_text(module);
		logln!("Start work on module '{module}'...");

		for task in 0..TASKS {
			lower.set(task, format!("task{}", task));
			sleep(Duration::from_millis(100));
			logln!("Finished task 'task{task}'");
		}

		logln!("Finished module '{module}'");
		lower.set_position(TASKS);
		upper.set_position(m as u64 + 1);
	}

	trace!("End do_tasks()");
}
