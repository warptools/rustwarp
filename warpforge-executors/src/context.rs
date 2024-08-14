use std::path::PathBuf;

#[derive(Clone, Default, Debug)]
pub struct Context {
	/// Path to OCI Runtime executable used to run containers in this context.
	pub runtime: PathBuf,

	/// Absolute path that determines the host path of mounts.
	/// This is used as the prefix, when a formula specifies a relative mount path.
	///
	/// If no [Self::mount_path] is configured, the formula must not use relative mount paths.
	pub mount_path: Option<PathBuf>,
}
