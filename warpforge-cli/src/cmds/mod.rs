pub mod catalog;
pub mod ware;

#[derive(clap::Parser, Debug)]
pub struct Root {
	#[command(subcommand)]
	pub subcommand: Option<Subcommands>,

	/// Raise verbosity by specifying this flag repeatedly.
	#[arg(short, action = clap::ArgAction::Count)]
	pub verbosity: u8,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommands {
	/// Docs go here.
	Catalog(catalog::Cmd),
	Ware(ware::Cmd),
}
