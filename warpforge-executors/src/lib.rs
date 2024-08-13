use std::path::{Path, PathBuf};

use indexmap::IndexMap;

mod errors;
mod events;
pub mod execute;
pub mod formula;
mod oci;

#[cfg(test)]
mod test;

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
	pub fn new_overlayfs(
		dest: impl AsRef<Path>,
		lowerdir: impl AsRef<Path>,
		upperdir: impl AsRef<Path>,
		workdir: impl AsRef<Path>,
	) -> Self {
		MountSpec {
			destination: to_string_or_panic(dest),
			kind: "overlayfs".into(),
			source: "none".into(),
			options: vec![
				format!("lowerdir={}", to_string_or_panic(lowerdir)),
				format!("upperdir={}", to_string_or_panic(upperdir)),
				format!("workdir={}", to_string_or_panic(workdir)),
			],
		}
	}

	pub fn new_bind(path: impl AsRef<Path>, dest: impl AsRef<Path>, read_only: bool) -> Self {
		let mut options = vec!["rbind".into()];
		if read_only {
			options.push("ro".into())
		};
		MountSpec {
			destination: to_string_or_panic(dest),
			kind: "none".into(),
			source: to_string_or_panic(path),
			options,
		}
	}
}
