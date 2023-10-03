#[derive(clap::Args, Debug)]
pub struct Cmd {
	#[command(subcommand)]
	pub subcommand: Subcommands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommands {
	/// unpack fills a directory with files by unpackaging a packed fileset (a "ware" -- may be a tarball, or refer to a git repo, etc)
	/// which you specify by use of a wareID (e.g. looks like "tar:asdfweiufh" or "git:1234cdf").
	/// It is designed to be used easily within shell scripts.
	///
	/// Optional flags to the command can specify the unpacking location or and whether to overwrite existing files.
	///
	/// Internally, this command may use `rio` or other plugin subcommands, but this detail is generally hidden from the user.
	Unpack(UnpackCmdArgs),
}

use std::str::FromStr;

#[derive(clap::Args, Debug)]
pub struct UnpackCmdArgs {
	#[arg(value_parser = warpforge_api::content::WareID::from_str)]
	pub ware_id: warpforge_api::content::WareID,

	#[arg(long = "fetch-url")]
	pub fetch_url: Vec<String>,
	#[arg(short, long = "dest", value_name = "DIR")]
	pub dest: String,
}
