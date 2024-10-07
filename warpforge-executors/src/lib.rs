use std::fs;
use std::path::{Path, PathBuf};

use context::Context;
use indexmap::IndexMap;

pub mod context;
mod errors;
mod events;
pub mod execute;
pub mod formula;
mod oci;
mod pack;
pub mod plot;

#[cfg(test)]
mod tests;

pub use errors::Error;
pub use errors::Result;
pub use events::Event;

/// This struct contains most of the parameters of a container execution that vary in Warpforge.
/// It's lower-level than a Formula (we never expose this API to users).
///
/// The main difference is that all mount instructions are turned into local paths already.
/// So, no more WareIDs down here.  Ware manifestation must have already happened, etc.
/// Generating any tempdirs for overlayFSes also should've happened already.
pub struct ContainerParams {
	ident: String,
	/// OCI compatible container runtime.
	runtime: PathBuf,
	command: Vec<String>,
	/// Mounts, mapped by destination.
	mounts: IndexMap<String, MountSpec>,
	environment: IndexMap<String, String>,
	root_path: PathBuf,
}

#[derive(PartialEq, Hash, Clone, Debug)]
pub struct Output {
	pub name: String,
	pub digest: Digest,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Digest {
	Sha384(String),
}

pub struct MountSpec {
	/// The destination mount path.  Should be absolute.
	destination: String,

	/// Typical mount types include "overlay", "tmpfs", "rbind".
	kind: String,

	/// Often, "none", or repeats the kind.
	/// For bind mounts, this is another path.
	source: String,

	/// Freetext, more or less.
	/// What exactly this means depends on the mount type, and is processed by that particular subsystem.
	///
	/// We present this as a list, but in the bottom of the world, it's comma separated strings
	/// that go after a colon-separated string, so... don't try to use those characters.
	/// (Or do, we're not your boss; just be prepared for how it's (not) going to be handled.)
	///
	/// For overlayfs, several paths go in here.  Consider using our helpful constructor for munging those
	/// (but ultimately it's just syntactic sugar for composing options strings).
	options: Vec<String>,
}

/// Since paths originate from json or rs files, they should always be UTF-8.
/// If an user tries to use non-UTF-8 paths, this should be detected at json deserialization.
fn to_string_or_panic(path: impl AsRef<Path>) -> String {
	path.as_ref()
		.to_str()
		.expect("encountered non-UTF-8 path")
		.into()
}

impl MountSpec {
	fn to_absolute(context: &Context, path: impl AsRef<Path>) -> Result<String> {
		let source = if path.as_ref().is_absolute() {
			to_string_or_panic(path)
		} else {
			let path = match &context.mount_path {
				Some(mount_path) if mount_path.is_absolute() => mount_path.join(path),
				_ => {
					return Err(Error::SystemSetupCauseless { msg: "failed to create mount: relative paths require context to provide absolute mount path".into() });
				}
			};
			path.to_str().ok_or_else(|| Error::SystemSetupCauseless { msg: "non-UTF-8 characters in mount path: to use relative mount paths, the path to the .wf file must be UTF-8".into() })?.to_string()
		};
		Ok(source)
	}

	pub fn new_overlayfs(
		context: &Context,
		path: impl AsRef<Path>,
		dest: impl AsRef<Path>,
		run_dir: impl AsRef<Path>,
	) -> Result<Self> {
		let destination = to_string_or_panic(dest);

		let mount_id = format!("mount{}", destination.replace('/', "-"));
		let overlay_dir = run_dir.as_ref().join("overlays").join(mount_id);
		if overlay_dir.exists() {
			let msg = "directory already existed when trying to setup overlayfs directories".into();
			return Err(Error::SystemSetupCauseless { msg });
		}

		let upperdir = overlay_dir.join("upper");
		let workdir = overlay_dir.join("work");
		fs::create_dir_all(&upperdir)
			.and_then(|_| fs::create_dir(&workdir))
			.map_err(|err| Error::SystemSetupError {
				msg: "failed to create upperdir and workdir directories for overlayfs mount".into(),
				cause: Box::new(err),
			})?;

		Ok(MountSpec {
			destination,
			kind: "overlay".into(),
			source: "none".into(),
			options: vec![
				format!("lowerdir={}", Self::to_absolute(context, path)?),
				format!("upperdir={}", to_string_or_panic(upperdir)),
				format!("workdir={}", to_string_or_panic(workdir)),
			],
		})
	}

	pub fn new_bind(
		context: &Context,
		path: impl AsRef<Path>,
		dest: impl AsRef<Path>,
		read_only: bool,
	) -> Result<Self> {
		let mut options = vec!["rbind".into()];
		if read_only {
			options.push("ro".into())
		};
		Ok(MountSpec {
			destination: to_string_or_panic(dest),
			kind: "none".into(),
			source: Self::to_absolute(context, path)?,
			options,
		})
	}
}
