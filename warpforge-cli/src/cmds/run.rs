use std::{
	env::current_dir,
	fs::{self, File},
	io::BufReader,
	path::{Path, PathBuf},
};

use warpforge_api::{constants::MAGIC_FILENAME_MODULE, formula::FormulaAndContext};
use warpforge_executors::formula::run_formula;

use crate::{cmds::Root, Error};

#[derive(clap::Args, Debug)]
pub struct Cmd {
	/// Path to formula file or module folder.
	///
	/// If no target is provided, run tries to target the current/working directory (cwd)
	/// as a module. An error is reported if no module is found.
	pub target: Option<PathBuf>,
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
		execute_formula(cli, target).await
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

async fn execute_formula(_cli: &Root, path: impl AsRef<Path>) -> Result<(), Error> {
	let file = File::open(path).map_err(|e| Error::InvalidArguments { cause: Box::new(e) })?;
	let reader = BufReader::new(file);
	let formula: FormulaAndContext =
		serde_json::from_reader(reader).map_err(|e| Error::InvalidArguments {
			cause: format!("invalid formula file: {e}").into(),
		})?;

	Ok(run_formula(formula).await?)
}
