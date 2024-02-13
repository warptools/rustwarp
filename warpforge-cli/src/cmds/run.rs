use std::ffi::OsString;

use crate::{cmds::Root, Error};

#[derive(clap::Args, Debug)]
pub struct Cmd {
	/// Path to formula file or module folder.
	///
	/// If no target is provided, run tries to target the current/working directory (cwd)
	/// as a module. An error is reported if no module is found.
	pub target: Option<OsString>,
}

pub fn execute(_cli: &Root, cmd: &Cmd) -> Result<(), Error> {
	Ok(())
}
