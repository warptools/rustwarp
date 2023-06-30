#[derive(clap::Subcommand, Debug)]
pub enum Cmd {
	/// read-item is a plumbing command.  It looks up a value by "{moduleName}:{releaseName}:{itemName}" tuple and returns the result as a plain string.
	/// It is designed to be used easily within shell scripts.
	///
	/// Optional flags to the command can cause additonal data to be reported with line-break delimiters, or cause the command to operate in JSON mode.
	ReadItem(ReadItemCmdArgs),
}

use std::str::FromStr;

#[derive(clap::Args, Debug)]
pub struct ReadItemCmdArgs {
	#[arg(value_parser = warpforge_api::catalog::CatalogRef::from_str)]
	catalog_ref: warpforge_api::catalog::CatalogRef,
}
