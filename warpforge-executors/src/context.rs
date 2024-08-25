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

	/// Path where outputs of a formula will be emitted.
	///
	/// If no [Self::output_path] is provided, the outputs will be created in the working directory.
	pub output_path: Option<PathBuf>,

	/// Path to the image cache.
	///
	/// If no [Self::image_cache] is specified, images are always pulled freshly from the registry.
	pub image_cache: Option<PathBuf>,
}
