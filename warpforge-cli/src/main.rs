use clap::Parser;
use std::env;
use std::path;

mod cmds;
mod dab;

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
					match env::var("HOME") {
						Ok(val) => {
							// Create the catalog data access broker.  Store in a box just so we can have dynamic dispatch.  (This is architecture astronauting, but I wanna know that I know how to do this.)
							//TODO: check for a root workspace above $CWD before $HOME/.warphome
							let catalog_handle: Box<dyn dab::catalog::Handle> =
								Box::new(dab::catalog::FsHandle::new(
									path::Path::new(&val).join(".warphome/catalogs/warpsys"),
								));
							let catalog_release = catalog_handle.load_release(
								&cmd.catalog_ref.module_name,
								&cmd.catalog_ref.release_name,
							);
							match catalog_release {
								Ok(cr) => match cr.items.get(&cmd.catalog_ref.item_name) {
									Some(wareid) => println!("{wareid}"),
									None => println!("catalog item not found."),
									// TODO: return non-zero
								},

								Err(e) => {
									println!("Failed to load_release from catalog_handle ({e})")
									// TODO: return non-zero
								}
							}
						}
						Err(e) => println!("$HOME not set! ({e}) Failing."),
						// TODO: return non-zero
					}
				}
			}
		}
		Some(cmds::Subcommands::Ware(cmd)) => match &cmd.subcommand {
			cmds::ware::Subcommands::Unpack(cmd) => {
				println!("unpack unimplemented...")
			}
		},
		None => {
			println!("command used with no args.  some explanation text should go here :)");
			// TODO: return non-zero
		}
	}
}

#[test]
fn verify_cli() {
	use clap::CommandFactory;
	cmds::Root::command().debug_assert()
}
