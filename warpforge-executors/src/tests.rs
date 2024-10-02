use std::{env, path::PathBuf};

use tempfile::TempDir;
use tokio::sync::mpsc;
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
	let image_cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.images");
	Context {
		runtime: "runc".into(),
		image_cache: Some(image_cache),
		..Default::default()
	}
}

async fn run_formula_collect_output(
	formula_and_context: FormulaAndContext,
	context: &Context,
) -> Result<RunOutput> {
	let tempdir = TempDir::new().unwrap();
	let executor = Executor {
		ersatz_dir: tempdir.path().join("run"),
		log_file: tempdir.path().join("log"),
	};
	let (gather_chan, mut gather_chan_recv) = mpsc::channel::<Event>(32);

	let formula = Formula { executor, context };

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

		RunOutput {
			exit_code,
			console: outputs,
			outputs: Vec::with_capacity(0),
		}
	});

	let outputs = formula.run(formula_and_context, gather_chan).await?;

	let mut run_output = gather_handle.await.expect("gathering events failed");
	run_output.outputs = outputs;
	Ok(run_output)
}
