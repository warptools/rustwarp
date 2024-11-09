use std::{
	env::current_dir,
	ffi::OsStr,
	fs::{self, File},
	io::BufReader,
	path::{Path, PathBuf},
};

use warpforge_api::{
	constants::{MAGIC_FILENAME_MODULE, MAGIC_FILENAME_PLOT},
	plot::PlotCapsule,
};
use warpforge_executors::{context::Context, formula::run_formula, plot::run_plot, Digest};
use warpforge_terminal::{log_global, logln, Level};
use warpforge_validate::validate_formula;

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
	let source = fs::read_to_string(&path).map_err(|err| {
		let cause = format!("failed to read formula file: {err}").into();
		Error::InvalidArguments { cause }
	})?;

	let result = validate_formula(&source);
	let validated_formula = match result {
		Ok(formula) => formula,
		Err(err) => {
			display_error(&err, &source, &path);
			let cause = format!("invalid formula file: {err}").into();
			return Err(Error::InvalidArguments { cause });
		}
	};

	let parent = parent(&path)?;
	let context = Context {
		runtime: cmd.runtime.to_owned(),
		mount_path: Some(parent),
		..Default::default()
	};
	let outputs = run_formula(validated_formula.formula, &context)?;

	for output in outputs {
		let warpforge_executors::Output {
			name,
			digest: Digest::Sha384(digest),
		} = output;
		logln!("sha384:{digest} {name}");
	}

	Ok(())
}

fn display_error(err: &warpforge_validate::Error, source: &str, path: impl AsRef<Path>) {
	use ariadne::{ColorGenerator, IndexType, Label, Report, ReportKind, Source};

	let warpforge_validate::Error::Invalid { errors } = err;

	let file_name = (path.as_ref().file_name())
		.and_then(OsStr::to_str)
		.unwrap_or("");
	let color_primary = ColorGenerator::new().next();

	let trailing_errors: Vec<_> = (errors.iter())
		.filter(|err| err.is_trailing_comma())
		.collect();
	if !trailing_errors.is_empty() {
		let first_span = (trailing_errors.iter())
			.filter_map(|err| err.span(source))
			.next()
			.unwrap_or_default();
		let mut report = Report::build(ReportKind::Error, (file_name, first_span))
			.with_config(ariadne::Config::default().with_index_type(IndexType::Byte))
			.with_message("found trailing comma(s)");
		for trailing in trailing_errors {
			let Some(span) = trailing.span(source) else {
				continue;
			};
			report = report.with_label(
				Label::new((file_name, span))
					.with_message("trailing comma")
					.with_color(color_primary),
			);
		}

		print_ariadne_report(report.finish(), (file_name, Source::from(source)));
	}

	for err in errors.iter().filter(|err| !err.is_trailing_comma()) {
		let span = err.span(source);
		let mut report = Report::build(
			ReportKind::Error,
			(file_name, span.clone().unwrap_or_default()),
		)
		.with_config(ariadne::Config::default().with_index_type(IndexType::Byte))
		.with_message(format!("{err}"));

		if let Some(span) = span {
			if span == (0..0) {
				continue;
			}
			let label = err.label().unwrap_or("here");
			report = report.with_label(
				Label::new((file_name, span))
					.with_message(label)
					.with_color(color_primary),
			);
			if let Some(note) = err.note() {
				report = report.with_note(note);
			}
		}

		print_ariadne_report(report.finish(), (file_name, Source::from(source)));
	}
}

fn print_ariadne_report<S, C>(report: ariadne::Report<'_, S>, cache: C)
where
	S: ariadne::Span,
	C: ariadne::Cache<S::SourceId>,
{
	let mut message = Vec::new();
	let result = report.write(cache, &mut message);
	if result.is_err() {
		return;
	}
	message.push(b'\n');
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
