use std::path::PathBuf;

use oci_client::secrets::RegistryAuth;

#[derive(Clone, Debug)]
pub struct PullConfig {
	/// Cache directory which should be used.
	///
	/// [pull_image] and [pull_and_unpack] will try to get images from the cache first and
	/// only fetch them from the registry on a cache miss. After fetching images from the
	/// registry, they are stored in the cache.
	///
	/// If no cache is specified, the images are always fetched from the registry.
	pub cache: Option<PathBuf>,

	pub auth: RegistryAuth,
}

impl Default for PullConfig {
	fn default() -> Self {
		Self {
			cache: None,
			auth: RegistryAuth::Anonymous,
		}
	}
}
