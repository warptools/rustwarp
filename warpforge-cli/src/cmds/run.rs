use std::{
	env::current_dir,
	fs::{self, File},
	io::BufReader,
	path::{Path, PathBuf},
};

use warpforge_api::{constants::MAGIC_FILENAME_MODULE, formula::FormulaAndContext};
use warpforge_executors::{context::Context, formula::run_formula, Digest};
use warpforge_terminal::logln;

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

pub async fn execute(cli: &Root, cmd: &Cmd) -> Result<(), Error> {
	let Some(target) = &cmd.target else {
		let path = current_dir().map_err(|e| Error::BizarreEnvironment { cause: Box::new(e) })?;
		return execute_module(cli, &path).await;
	};

	let meta = fs::metadata(target).map_err(|e| Error::InvalidArguments { cause: Box::new(e) })?;
	if meta.is_dir() {
		execute_module(cli, target).await
	} else if meta.is_file() {
		execute_formula(cmd, target).await
	} else {
		Err(Error::InvalidArguments {
			cause: "invalid target: 'run' requires an existing file or directory".into(),
		})
	}
}

async fn execute_module(_cli: &Root, path: impl AsRef<Path>) -> Result<(), Error> {
	if !path.as_ref().join(MAGIC_FILENAME_MODULE).is_file() {
		return Err(Error::InvalidArguments {
			cause: format!(
				"invalid target: directory does not contain file '{}'",
				MAGIC_FILENAME_MODULE
			)
			.into(),
		});
	}

	todo!()
}

async fn execute_formula(cmd: &Cmd, path: impl AsRef<Path>) -> Result<(), Error> {
	let file = File::open(&path).map_err(|e| Error::InvalidArguments { cause: Box::new(e) })?;
	let reader = BufReader::new(file);
	let formula: FormulaAndContext =
		serde_json::from_reader(reader).map_err(|e| Error::InvalidArguments {
			cause: format!("invalid formula file: {e}").into(),
		})?;

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
	let parent = parent.ok_or_else(|| Error::BizarreEnvironment {
		cause: "could not get parent of formula file after successfully reading it".into(), // has to be race condition
	})?;

	let context = Context {
		runtime: cmd.runtime.to_owned(),
		mount_path: Some(parent),
		..Default::default()
	};
	let outputs = run_formula(formula, &context).await?;

	for output in outputs {
		let warpforge_executors::Output {
			name,
			digest: Digest::Sha384(digest),
		} = output;
		logln!("sha384:{digest} {name}");
	}

	Ok(())
}
