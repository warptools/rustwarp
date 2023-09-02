use clap::Parser;

mod dab;
mod cmds;

fn main() {
	let cli = cmds::Root::parse();

	if cli.verbosity >= 2 {
		println!("args: {:?}", cli);
	}

	// Dispatch.
	//
	// Okay, I have some nonjoy at this.  I want:
	//   - 1: to receive the command object with all parents.
	//   - 2: to have a func on my command strugs that receives a call, rather than have to make this dispatch table.
	match &cli.subcommand {
		Some(cmds::Subcommands::Catalog(cmd)) => {
			match &cmd.subcommand {
				cmds::catalog::Subcommands::ReadItem(cmd) => {
					println!("args: {:?}", cmd.catalog_ref);
					// Create the catalog data access broker.  Store in a box just so we can have dynamic dispatch.  (This is architecture astronauting, but I wanna know that I know how to do this.)
					// TODO: no, $HOME here is not valid; find a rust crate or sjust read the var or something for that.
					let catalog_handle : Box<dyn dab::catalog::Handle> = Box::new(dab::catalog::FsHandle::new("$HOME/.warphome/catalogs/warpsys"));
				}
			}
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
