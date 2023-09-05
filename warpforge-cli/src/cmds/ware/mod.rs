#[derive(clap::Args, Debug)]
pub struct Cmd {
	#[command(subcommand)]
	pub subcommand: Subcommands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommands {
	/// unpack is a plumbing command.  It accepts a WareID and calls `rio` to unpack the tarball in the cwd or specified path.
	/// It is designed to be used easily within shell scripts.
	///
	/// Optional flags to the command can specify the unpacking location or and whether to overwrite existing files.
	Unpack(UnpackCmdArgs),
}

use std::str::FromStr;

#[derive(clap::Args, Debug)]
pub struct UnpackCmdArgs {
	#[arg(value_parser = warpforge_api::content::WareID::from_str)]
	pub ware_id: warpforge_api::content::WareID,
}
