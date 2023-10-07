use std::ffi::{OsStr, OsString};
use std::path::Path;

use indexmap::IndexMap;

mod execdemo;
mod gvisor;
mod oci;

/// This struct contains most of the parameters of a container execution that vary in Warpforge.
/// It's lower-level than a Formula (we never expose this API to users).
///
/// The main difference is that all mount instructions are turned into local paths already.
/// So, no more WareIDs down here.  Ware manifestation must have already happened, etc.
/// Generating any tempdirs for overlayFSes also should've happened already.
pub struct ContainerParams {
	// TODO
	// Probably the wfapi::Action structure just comes straight through; don't see why not.

	/// Mounts, mapped by destination.
	mounts: IndexMap<OsString, MountSpec>
}

pub struct MountSpec {
	/// The destination mount path.  Should be absolute.
	destination: OsString,

	/// Typical mount types include "overlay", "tmpfs", "rbind".
	kind: OsString,

	/// Often, "none", or repeats the kind.
	/// For bind mounts, this is another path.
	source: OsString,

	/// Freetext, more or less.
	/// What exactly this means depends on the mount type, and is processed by that particular subsystem.
	///
	/// We present this as a list, but in the bottom of the world, it's comma separated strings
	/// that go after a colon-separated string, so... don't try to use those characters.
	/// (Or do, we're not your boss; just be prepared for how it's (not) going to be handled.)
	///
	/// For overlayfs, several paths go in here.  Consider using our helpful constructor for munging those
	/// (but ultimately it's just syntactic sugar for composing options strings).
	options: Vec<OsString>,
}

impl MountSpec {
	pub fn new_overlayfs(dest: &Path, lowerdir: &Path, upperdir: &Path, workdir: &Path) -> Self {
		return MountSpec {
			destination: dest.as_os_str().to_owned(),
			kind: OsString::from("overlayfs"),
			source: OsString::from("none"),
			options: vec![
				// Holy smokes string ops in rust are spicy.
				// Path is not constrained to UTF8, and neither is OsString, so we're staying in those two.
				// But concatenation isn't really implemented on OsString, at least as far as I can tell while writing this.
				// We implemented our own.  If I'm wrong and this is in stdlib, plz, PR.  I'm not proud of this.
				//
				// Mind, all of this is a huge farce, because we're going to end up passing these around in JSON anyway.
				// (For any of the OCI-based executors, that's how we communicate with them.)
				// And JSON doesn't support non-UTF string sequences.
				// Whoopsie.
				// Nonetheless: I do like as much of the code as possible to be correct in handling sequences losslessly.
				concat_to_osstring("lowerdir=", lowerdir),
				concat_to_osstring("upperdir=", upperdir),
				concat_to_osstring("workdir=", workdir),
			],
		};
	}
}

// future work: make this into a `impl Add<AsRef<OsStr>> for OsString`?
fn concat_to_osstring<T: AsRef<OsStr> + ?Sized, U: AsRef<OsStr> + ?Sized>(
	a: &T,
	b: &U,
) -> OsString {
	let mut ret = OsString::with_capacity(a.as_ref().len() + b.as_ref().len());
	ret.push(a);
	ret.push(b);
	ret
}
