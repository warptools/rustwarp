use clap::{Args, Parser, Subcommand};
use warpforge_api;

mod cmds;

fn main() {
	let cli = cmds::Root::parse();

	if cli.verbosity >= 2 {
		println!("args: {:?}", cli);
	}

	match &cli.command {
		Some(cmds::Subcommands::Catalog(args)) => {
			let expect_to_load_this_from_files: Option<warpforge_api::compute::Workflow>;
		}
		None => {
			println!("command used with no args.  some explanation text should go here :)");
		}
	}
}

#[test]
fn verify_cli() {
	use clap::CommandFactory;
	cmds::Root::command().debug_assert()
}
