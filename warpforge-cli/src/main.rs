use clap::{Args, Parser, Subcommand};
use warpforge_api;

mod cmds;

fn main() {
	let cli = Cli::parse();

	println!("args: {:?}", cli);

	match &cli.command {
		Some(Subcommands::Eval(args)) => {
			let expect_to_load_this_from_files: Option<warpforge_api::compute::Workflow>;
		}
		Some(Subcommands::Boogie(args)) => {}
		None => {}
	}
}

#[test]
fn verify_cli() {
	use clap::CommandFactory;
	Cli::command().debug_assert()
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[derive(Debug)]
struct Cli {
	#[command(subcommand)]
	command: Option<Subcommands>,

	/// Raise verbosity by specifying this flag repeatedly.
	#[arg(short, action = clap::ArgAction::Count)]
	verbosity: u8,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
	/// Docs go here.
	Eval(EvalArgs),
	/// Docs for this subcommand.
	Boogie(BoogieArgs),
}

#[derive(Args, Debug)]
struct EvalArgs {
	path_patterns: Vec<String>,
}

#[derive(Args, Debug)]
struct BoogieArgs {
	string: Option<String>,
}
