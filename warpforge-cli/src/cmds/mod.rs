pub mod catalog;

#[derive(clap::Parser, Debug)]
struct Root {
	#[command(subcommand)]
	command: Option<Subcommands>,

	/// Raise verbosity by specifying this flag repeatedly.
	#[arg(short, action = clap::ArgAction::Count)]
	verbosity: u8,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommands {
	/// Docs go here.
	Catalog(catalog::Cmd),
}
