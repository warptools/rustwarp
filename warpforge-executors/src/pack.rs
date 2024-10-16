use std::{
	fs::{self, File},
	io::{BufWriter, Write},
	path::{Path, PathBuf},
};

use oci_unpack::tee::WriteExt;
use sha2::{Digest, Sha384};
use warpforge_api::content::Packtype;

use crate::{Error, Output, Result};

pub(crate) struct IntermediateOutput {
	pub(crate) name: String,
	pub(crate) host_path: PathBuf,
	pub(crate) packtype: OutputPacktype,
}

pub(crate) enum OutputPacktype {
	None,
	Tar,
}

impl OutputPacktype {
	pub(crate) fn parse(packtype: &Option<Packtype>) -> Result<Self> {
		Ok(match packtype {
			None => OutputPacktype::None,
			Some(Packtype(p)) if p == "none" => OutputPacktype::None,
			Some(Packtype(p)) if p == "tar" => OutputPacktype::Tar,
			_ => {
				let msg = "unsupported packtype (allowed values: 'none', 'tar')".into();
				return Err(Error::SystemSetupCauseless { msg });
			}
		})
	}
}

pub(crate) fn pack_outputs(
	output_dir: &Option<PathBuf>,
	outputs: &[IntermediateOutput],
) -> Result<Vec<Output>> {
	if outputs.is_empty() {
		return Ok(Vec::with_capacity(0)); // exit early without allocations.
	}

	let mut results = Vec::new();

	let target_dir = output_dir.clone().unwrap_or_default();
	fs::create_dir_all(&target_dir).map_err(|err| Error::SystemRuntimeError {
		msg: "failed to create directory".into(),
		cause: Box::new(err),
	})?;

	for output in outputs {
		let IntermediateOutput {
			name,
			host_path,
			packtype,
		} = output;

		let target = target_dir.join(name);
		let output = match packtype {
			OutputPacktype::None => {
				// TODO: Handle ErrorKind::CrossesDevices: we should handle move between mounts.
				fs::rename(host_path, &target).map_err(|err| Error::SystemRuntimeError {
					msg: "failed to move output dir to target".into(),
					cause: Box::new(err),
				})?;
				tar_dir_hash_only(name, target)?
			}
			OutputPacktype::Tar => tar_dir_to_file(name, host_path, &target)?,
		};
		results.push(output);
	}

	Ok(results)
}

pub(crate) fn tar_dir_hash_only(name: &str, source_dir: impl AsRef<Path>) -> Result<Output> {
	let mut digester = Sha384::new();
	tar_dir(&source_dir, &mut digester)?;

	let digest = crate::Digest::Sha384(format!("{:x}", digester.finalize()));
	let name = name.to_owned();
	Ok(Output { name, digest })
}

pub(crate) fn tar_dir_to_file(
	name: &str,
	source_dir: impl AsRef<Path>,
	target_file: impl AsRef<Path>,
) -> Result<Output> {
	let writer = File::create(target_file)
		.map(BufWriter::new)
		.map_err(|err| Error::SystemRuntimeError {
			msg: "failed to create output file".into(),
			cause: Box::new(err),
		})?;

	let mut digester = Sha384::new();
	let writer = writer.tee(&mut digester);

	tar_dir(source_dir, writer)?;

	let digest = crate::Digest::Sha384(format!("{:x}", digester.finalize()));
	let name = name.to_owned();
	Ok(Output { name, digest })
}

pub(crate) fn tar_dir(source_dir: impl AsRef<Path>, writer: impl Write) -> Result<()> {
	let mut archive = tar::Builder::new(writer);
	archive.mode(tar::HeaderMode::Deterministic);
	archive
		.append_dir_all("", source_dir)
		.and_then(|_| archive.finish())
		.map_err(|err| Error::SystemRuntimeError {
			msg: "failed to pack output".into(),
			cause: Box::new(err),
		})
}
