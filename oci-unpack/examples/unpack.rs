use std::{
	env::{self},
	path::PathBuf,
	process::exit,
};

use oci_unpack::{pull_and_unpack, PullConfig};

#[tokio::main]
async fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 3 {
		eprintln!("usage: {} <image> <target>", args[0]);
		eprintln!("  <image>   Reference to image including registry-id.");
		eprintln!("  <target>  Target directory for unpack operation.");
		exit(64); // EX_USAGE=64: The command was used incorrectly. (See sysexits.h)
	}

	let reference = match args[1].parse() {
		Ok(reference) => reference,
		Err(error) => {
			eprintln!("image reference could no be parsed: {error}");
			exit(65); // EX_DATAERR (65): The input data was incorrect in some way. (See sysexits.h)
		}
	};

	let target: PathBuf = args[2].clone().into();

	match pull_and_unpack(&reference, &target, &PullConfig::default()).await {
		Ok(info) => println!("{:#?}", info.manifest),
		Err(error) => {
			eprintln!("unpacking failed: {error}");
			exit(1); // Unknown error.
		}
	};
}
