use std::{
	env::current_dir,
	ffi::OsStr,
	fs::{self, File},
	io::BufReader,
	path::{Path, PathBuf},
};

use ariadne::{ColorGenerator, Label, Source};
use serde_json::error::Category;
use warpforge_api::{
	constants::{MAGIC_FILENAME_MODULE, MAGIC_FILENAME_PLOT},
	formula::FormulaAndContext,
	plot::PlotCapsule,
};
use warpforge_executors::{context::Context, formula::run_formula, plot::run_plot, Digest};
use warpforge_terminal::{log_global, logln, Level};

use crate::{cmds::Root, Error};

#[derive(clap::Args, Debug)]
pub struct Cmd {
	/// Path to formula file or module folder.
	///
	/// If no target is provided, run tries to target the current/working directory (cwd)
	/// as a module. An error is reported if no module is found.
	pub target: Option<PathBuf>,

	/// Container runtime used to run OCI bundles.
	#[arg(long, default_value = "runc")]
	pub runtime: PathBuf,
}

pub fn execute(_cli: &Root, cmd: &Cmd) -> Result<(), Error> {
	let Some(target) = &cmd.target else {
		let path = current_dir().map_err(|e| Error::BizarreEnvironment { cause: Box::new(e) })?;
		return execute_module(cmd, path);
	};

	let meta = fs::metadata(target).map_err(|e| Error::InvalidArguments { cause: Box::new(e) })?;
	if meta.is_dir() {
		execute_module(cmd, target)
	} else if meta.is_file() {
		execute_formula(cmd, target)
	} else {
		Err(Error::InvalidArguments {
			cause: "invalid target: 'run' requires an existing file or directory".into(),
		})
	}
}

fn execute_module(cmd: &Cmd, path: impl AsRef<Path>) -> Result<(), Error> {
	if !path.as_ref().join(MAGIC_FILENAME_MODULE).is_file() {
		return Err(Error::InvalidArguments {
			cause: format!(
				"invalid target: directory does not contain file '{MAGIC_FILENAME_MODULE}'",
			)
			.into(),
		});
	}

	let plot_path = path.as_ref().join(MAGIC_FILENAME_PLOT);
	let file = File::open(plot_path).map_err(|e| Error::InvalidArguments { cause: Box::new(e) })?;
	let reader = BufReader::new(file);
	let plot: PlotCapsule =
		serde_json::from_reader(reader).map_err(|e| Error::InvalidArguments {
			cause: format!("invalid plot file: {e}").into(),
		})?;

	let parent = parent(path)?;
	let context = Context {
		runtime: cmd.runtime.to_owned(),
		mount_path: Some(parent),
		..Default::default()
	};
	let outputs = run_plot(plot, &context)?;

	for output in outputs {
		let warpforge_executors::Output {
			name,
			digest: Digest::Sha384(digest),
		} = output;
		logln!("sha384:{digest} {name}");
	}

	Ok(())
}

fn execute_formula(cmd: &Cmd, path: impl AsRef<Path>) -> Result<(), Error> {
	let file = File::open(&path).map_err(|err| {
		let cause = format!("failed to open formula file: {err}").into();
		Error::InvalidArguments { cause }
	})?;
	let reader = BufReader::new(file);
	let formula: FormulaAndContext = match serde_json::from_reader(reader) {
		Ok(formula) => formula,
		Err(err) => match err.classify() {
			Category::Io => {
				let cause = format!("failed to read formula file: {err}").into();
				return Err(Error::InvalidArguments { cause });
			}
			Category::Eof => {
				let cause = format!("failed to parse formula file: {err}").into();
				return Err(Error::InvalidArguments { cause });
			}
			Category::Syntax | Category::Data => {
				display_error(path, &err);
				let cause = format!("invalid formula file: {err}").into();
				return Err(Error::InvalidArguments { cause });
			}
		},
	};

	let parent = parent(path)?;
	let context = Context {
		runtime: cmd.runtime.to_owned(),
		mount_path: Some(parent),
		..Default::default()
	};
	let outputs = run_formula(formula, &context)?;

	for output in outputs {
		let warpforge_executors::Output {
			name,
			digest: Digest::Sha384(digest),
		} = output;
		logln!("sha384:{digest} {name}");
	}

	Ok(())
}

fn display_error(path: impl AsRef<Path>, err: &serde_json::Error) {
	use ariadne::{Report, ReportKind};

	if err.line() < 1 {
		return;
	}
	let Ok(source) = fs::read_to_string(path.as_ref()) else {
		return; // Return on error: Not worth handling further errors at this point.
	};

	let line_offset: usize = source
		.split_inclusive('\n')
		.take(err.line() - 1)
		.map(|line| line.chars().count())
		.sum();
	let offset = line_offset + err.column() - 1;
	let mut error_range = offset..offset;
	let mut label = "here";

	// Find trailing comma, since serde_json points to closing braces instead of comma.
	// `serde_json::Error` does not allow us to match the concrete error kind,
	// so we look at the emitted error message.
	if err.is_syntax() && format!("{err}").contains("trailing comma") {
		let mut source: Vec<_> = source.chars().take(offset).collect();
		while let Some(last) = source.pop() {
			if last == ',' {
				error_range = source.len()..source.len() + 1;
				label = "trailing comma";
				break;
			}
			if !last.is_whitespace() {
				break;
			}
		}
	}

	let file_name = path
		.as_ref()
		.file_name()
		.and_then(OsStr::to_str)
		.unwrap_or("");

	let color = ColorGenerator::new().next();
	let report = Report::build(ReportKind::Error, (file_name, error_range.clone()))
		.with_message(format!("{err}"))
		.with_label(
			Label::new((file_name, error_range))
				.with_message(label)
				.with_color(color),
		)
		.finish();

	let mut message = Vec::new();
	let result = report.write((file_name, Source::from(source)), &mut message);
	if result.is_err() {
		return;
	}
	if let Ok(message) = String::from_utf8(message) {
		log_global(Level::Error, message);
	}
}

fn parent(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
	let parent = if path.as_ref().is_absolute() {
		path.as_ref().parent().map(ToOwned::to_owned)
	} else {
		(path.as_ref().canonicalize())
			.map_err(|err| Error::BizarreEnvironment {
				cause: Box::new(err),
			})?
			.parent()
			.map(ToOwned::to_owned)
	};
	parent.ok_or_else(|| Error::BizarreEnvironment {
		cause: "could not get parent after successfully accessing child-path".into(), // has to be race condition
	})
}
