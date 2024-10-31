use std::{env, path::PathBuf, thread};

use tempfile::TempDir;
use warpforge_api::formula::FormulaAndContext;

use crate::{
	context::Context, events::EventBody, execute::Executor, formula::Formula, Event, Output, Result,
};

mod formula;
mod plot;

#[derive(PartialEq, Debug)]
struct RunOutput {
	exit_code: Option<i32>,
	console: Vec<RunOutputLine>,
	outputs: Vec<Output>,
}

#[derive(PartialEq, Debug)]
struct RunOutputLine {
	channel: i32,
	line: String,
}

fn default_context() -> Context {
	let runtime = env::var("WARPFORGE_TEST_RUNTIME")
		.unwrap_or("runc".into())
		.into();
	let image_cache = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.images"));
	Context {
		runtime,
		image_cache,
		..Default::default()
	}
}

fn run_formula_collect_output(
	formula_and_context: FormulaAndContext,
	context: &Context,
) -> Result<RunOutput> {
	let tempdir = TempDir::new().unwrap();
	let executor = Executor {
		ersatz_dir: tempdir.path().join("run"),
		log_file: tempdir.path().join("log"),
	};
	let (gather_chan, gather_chan_recv) = crossbeam_channel::bounded::<Event>(32);

	let formula = Formula { executor, context };

	let gather_handle = thread::spawn(move || {
		let mut outputs = Vec::new();
		let mut exit_code = None;

		while let Ok(evt) = gather_chan_recv.recv() {
			match evt.body {
				EventBody::Output { channel, val: line } => {
					println!("[container:{channel}] {line}");
					outputs.push(RunOutputLine { channel, line });
				}
				EventBody::ExitCode(code) => {
					println!("[container-exit] {code:?}");
					exit_code = code;
				}
			};
		}

		RunOutput {
			exit_code,
			console: outputs,
			outputs: Vec::with_capacity(0),
		}
	});

	let outputs = formula.run(formula_and_context, gather_chan)?;

	let mut run_output = gather_handle.join().expect("gathering events failed");
	run_output.outputs = outputs;
	Ok(run_output)
}
