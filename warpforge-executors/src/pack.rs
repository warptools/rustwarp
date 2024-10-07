use std::{
	fs::File,
	io::{BufWriter, Write},
	path::Path,
};

use oci_unpack::tee::WriteExt;
use sha2::{Digest, Sha384};

use crate::{Error, Output, Result};

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
