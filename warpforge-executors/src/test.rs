use tempfile::TempDir;
use tokio::sync::mpsc;
use warpforge_api::formula::FormulaAndContext;

use crate::{events::EventBody, execute::Executor, formula::Formula, Event, Result};

mod simple_echo;
mod simple_mount;

#[derive(PartialEq, Debug)]
struct RunOutput {
	exit_code: Option<i32>,
	outputs: Vec<RunOutputLine>,
}

#[derive(PartialEq, Debug)]
struct RunOutputLine {
	channel: i32,
	line: String,
}

async fn run_formula_collect_output(formula_and_context: FormulaAndContext) -> Result<RunOutput> {
	let tempdir = TempDir::new().unwrap();
	let executor = Executor {
		ersatz_dir: tempdir.path().join("run"),
		log_file: tempdir.path().join("log"),
	};
	let (gather_chan, mut gather_chan_recv) = mpsc::channel::<Event>(32);

	let formula = Formula { executor };

	let gather_handle = tokio::spawn(async move {
		let mut outputs = Vec::new();
		let mut exit_code = None;

		while let Some(evt) = gather_chan_recv.recv().await {
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

		RunOutput { exit_code, outputs }
	});

	formula
		.run(formula_and_context, "runc".into(), gather_chan)
		.await?;

	Ok(gather_handle.await.expect("gathering events failed"))
}
