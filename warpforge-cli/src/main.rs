use clap::error::ErrorKind;
use clap::Parser;
use std::env;
use std::path;

use warpforge_terminal::logln;
use warpforge_terminal::Logger;

mod cmds;
mod dab;
mod errors;

use errors::*;

#[tokio::main]
async fn main() {
	Logger::set_global(Logger::new_local()).unwrap();

	let result = main2().await;
	if let Err(e) = &result {
		logln!("{}", e);
	}

	// Wait for all messages to be printed to stdout.
	let _ = Logger::get_global().unwrap().close().await;

	if let Err(e) = &result {
		std::process::exit(e.code());
	}
}

async fn main2() -> Result<(), Error> {
	let cli = match cmds::Root::try_parse() {
		Ok(arguments) => arguments,
		Err(e) if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) => {
			logln!("{e}");
			return Ok(());
		}
		Err(e) => return Err(Error::InvalidArguments { cause: Box::new(e) }),
	};

	if cli.verbosity >= 2 {
		logln!("args: {cli:?}");
	}

	// Dispatch.
	//
	// Okay, I have some nonjoy at this.  I want:
	//   - 1: to receive the command object with all parents.
	//   - 2: to have a func on my command strugs that receives a call, rather than have to make this dispatch table.
	match &cli.subcommand {
		Some(cmds::Subcommands::Run(cmd)) => return cmds::run::execute(&cli, cmd).await,
		Some(cmds::Subcommands::Catalog(cmd)) => match &cmd.subcommand {
			cmds::catalog::Subcommands::ReadItem(cmd) => {
				let user_home = env::var("HOME")
					.map_err(|e| Error::BizarreEnvironment { cause: Box::new(e) })?;

				// Create the catalog data access broker.  Store in a box just so we can have dynamic dispatch.  (This is architecture astronauting, but I wanna know that I know how to do this.)
				//TODO: check for a root workspace above $CWD before $HOME/.warphome
				let catalog_handle: Box<dyn dab::catalog::Handle> =
					Box::new(dab::catalog::FsHandle::new(
						path::Path::new(&user_home).join(".warphome/catalogs/warpsys"),
					));

				let catalog_release = catalog_handle
					.load_release(&cmd.catalog_ref.module_name, &cmd.catalog_ref.release_name)
					.map_err(|e| Error::CatalogAccess { cause: e })?;

				match catalog_release.items.get(&cmd.catalog_ref.item_name) {
					Some(wareid) => {
						logln!("{wareid}");
					}
					None => {
						logln!("catalog item not found.");
						return Err(Error::CatalogEntryNotExists {
							reference: cmd.catalog_ref.clone(),
						});
					}
				}
			}
		},
		Some(cmds::Subcommands::Ware(cmd)) => match &cmd.subcommand {
			cmds::ware::Subcommands::Unpack(cmd) => {
				use std::io::{BufRead, BufReader};
				use std::process::{Command, Stdio};
				let sources = cmd.fetch_url.iter().map(|s| "--source=".to_string() + s);
				let dest = match &cmd.dest {
					Some(s) => s,
					None => "unpack_default", // TODO find and use package name string
				};
				let mut riocmd = Command::new("rio");
				riocmd
					.args(["unpack", "--format=json", "--placer=direct"])
					.args(sources)
					.args([&cmd.ware_id.to_string(), dest]);
				// TODO implement destination flag
				// (and be careful cause rio will blow away anything in it's path!!!!!)
				let args: &Vec<_> = &riocmd.get_args().collect();
				logln!("Running \"rio {:?}\"", args);
				let mut child = riocmd
					.stdout(Stdio::piped())
					.stderr(Stdio::piped())
					.spawn()
					.unwrap();
				let mut child_out = BufReader::new(child.stdout.as_mut().unwrap());
				let mut line = String::new();

				BufRead::read_line(&mut child_out, &mut line).unwrap();
				logln!("{line}");
			}
		},
		Some(cmds::Subcommands::Graph(cmd)) => {
			warpforge_visualize::graph_dependencies(&cmd.package);
		}
		None => {
			logln!("command used with no args.  some explanation text should go here :)");
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn verify_cli() {
		use clap::CommandFactory;
		cmds::Root::command().debug_assert()
	}
}
