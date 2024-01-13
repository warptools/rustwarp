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
	/// subcommands for working with catalogs, warpforge's data labelling system.
	Catalog(catalog::Cmd),

	/// subcommands for working with wares and filesystems -- snapshotting, packing, unpacking, mirroring, etc.
	Ware(ware::Cmd),

	/// subcommand to graph dependencies of given package. A dot file is emitted to stdout.
	Graph(GraphCmd),
}

#[derive(clap::Args, Debug)]
pub struct GraphCmd {
	/// package for which the graph is created.
	pub package: String,
}
