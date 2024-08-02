//! Unpack [OCI Images] into [OCI Runtime Bundles].
//!
//! [OCI Images] are obtained by directly downloading them from a [OCI Registry] using the [oci-client] crate.
//! After obtaining the [OCI Images], they are then unpacked into [OCI Runtime Bundles].
//! Unpacking is done as similar as possible to the reference implementation [umoci].
//!
//! [OCI Images]: https://github.com/opencontainers/image-spec/blob/main/spec.md
//! [OCI Runtime Bundles]: https://github.com/opencontainers/runtime-spec/blob/main/bundle.md
//! [OCI Registry]: https://github.com/opencontainers/distribution-spec/blob/main/spec.md
//! [oci-client]: https://github.com/oras-project/rust-oci-client
//! [umoci]: https://github.com/opencontainers/umoci/blob/8e665b719d0aff18dbf97a287f78faa6d0ef4f18/unpack.go

use std::{fs, path::Path};

use oci_client::{
	client::ClientConfig,
	manifest::{IMAGE_LAYER_GZIP_MEDIA_TYPE, IMAGE_LAYER_MEDIA_TYPE},
	secrets::RegistryAuth,
	Client, Reference,
};

use crate::error::Result;

// TODO: Consider adding ZSTD support in addition to TAR and GZIP.
const LAYER_MEDIA_TYPES: [&str; 2] = [IMAGE_LAYER_MEDIA_TYPE, IMAGE_LAYER_GZIP_MEDIA_TYPE];

pub async fn unpack(
	image: &Reference,
	auth: &RegistryAuth,
	target: impl AsRef<Path>,
) -> Result<()> {
	let client = Client::new(ClientConfig::default());
	let image_data = client.pull(image, auth, LAYER_MEDIA_TYPES.to_vec()).await?;

	fs::create_dir_all(target)?;

	let _ = image_data; // TODO

	Ok(())
}

pub mod error {
	pub type Result<T> = std::result::Result<T, Error>;

	#[derive(thiserror::Error, Debug)]
	pub enum Error {
		#[error("failed to download image: {0}")]
		DownloadFailed(#[from] oci_client::errors::OciDistributionError),

		#[error("failed io operation: {0}")]
		IO(#[from] std::io::Error),
	}
}
