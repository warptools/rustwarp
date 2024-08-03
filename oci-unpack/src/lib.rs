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

use file_mode::ModePath;
use flate2::read::GzDecoder;
use oci_client::{
	client::{ClientConfig, ImageLayer},
	manifest::{
		OciImageManifest, IMAGE_DOCKER_LAYER_GZIP_MEDIA_TYPE, IMAGE_DOCKER_LAYER_TAR_MEDIA_TYPE,
		IMAGE_LAYER_GZIP_MEDIA_TYPE, IMAGE_LAYER_MEDIA_TYPE,
	},
	secrets::RegistryAuth,
	Client, Reference,
};
use oci_spec::image::ImageConfiguration;

use crate::error::{Error, Result};

// TODO: Consider adding ZSTD support in addition to TAR and GZIP.
const LAYER_MEDIA_TYPES: &[&str] = &[
	IMAGE_LAYER_MEDIA_TYPE,
	IMAGE_LAYER_GZIP_MEDIA_TYPE,
	IMAGE_DOCKER_LAYER_TAR_MEDIA_TYPE,
	IMAGE_DOCKER_LAYER_GZIP_MEDIA_TYPE,
];

fn is_gzip(media_type: &str) -> bool {
	media_type == IMAGE_LAYER_GZIP_MEDIA_TYPE || media_type == IMAGE_DOCKER_LAYER_GZIP_MEDIA_TYPE
}

pub struct BundleInfo {
	pub manifest: OciImageManifest,
	pub manifest_digest: String,
}

pub async fn unpack(
	image: &Reference,
	auth: &RegistryAuth,
	target: impl AsRef<Path>,
) -> Result<BundleInfo> {
	fs::create_dir_all(&target)?;
	let is_empty = target.as_ref().read_dir()?.next().is_none();
	if !is_empty {
		return Err(Error::TargetNotEmpty);
	}

	// From umoci:
	// "We change the mode of the bundle directory to 0700. A user can easily
	// change this after-the-fact, but we do this explicitly to avoid cases
	// where an unprivileged user could recurse into an otherwise unsafe image
	// (giving them potential root access through setuid binaries for example)."
	target.set_mode(0o700)?;

	let image_data = pull_image(image, auth).await?;

	let rootfs_dir = target.as_ref().join("rootfs");
	fs::create_dir(&rootfs_dir)?;
	// TODO: root UID and root GID mapping.
	// TODO: set atime/mtime of root directory.

	if image_data.config.rootfs().typ() != "layers" {
		return Err(Error::UnsupportedRootFSType {
			typ: image_data.config.rootfs().typ().to_string(),
		});
	}

	// TODO: Do we need to check diffIDs or are those the digests that are already checked by oci-client?
	for layer in image_data.layers {
		if is_gzip(&layer.media_type) {
			let decoder = GzDecoder::new(&layer.data[..]);
			tar::Archive::new(decoder).unpack(&rootfs_dir)?;
		} else {
			tar::Archive::new(&layer.data[..]).unpack(&rootfs_dir)?;
		}
	}

	Ok(BundleInfo {
		manifest: image_data.manifest,
		manifest_digest: image_data.manifest_digest,
	})
}

struct ImageData {
	manifest: OciImageManifest,
	manifest_digest: String,
	layers: Vec<ImageLayer>,
	config: ImageConfiguration,
}

async fn pull_image(image: &Reference, auth: &RegistryAuth) -> Result<ImageData> {
	let client = Client::new(ClientConfig::default());
	let image_data = client.pull(image, auth, LAYER_MEDIA_TYPES.to_vec()).await?;

	let manifest = image_data.manifest.unwrap();
	let manifest_digest = image_data.digest.unwrap();
	let config: ImageConfiguration =
		serde_json::from_slice(&image_data.config.data).map_err(Error::ParseImageConfiguration)?;

	Ok(ImageData {
		manifest,
		manifest_digest,
		layers: image_data.layers,
		config,
	})
}

pub mod error {
	pub type Result<T> = std::result::Result<T, Error>;

	#[derive(thiserror::Error, Debug)]
	pub enum Error {
		#[error("target directory was not empty")]
		TargetNotEmpty,

		#[error("failed to download image: {0}")]
		DownloadFailed(#[from] oci_client::errors::OciDistributionError),

		#[error("failed to parse image configuration: {0}")]
		ParseImageConfiguration(serde_json::Error),

		#[error("failed io operation: {0}")]
		IO(#[from] std::io::Error),

		#[error("failed to chmod: {0}")]
		ChangeMode(#[from] file_mode::ModeError),

		#[error("config: unsupported rootfs.type: {typ}")]
		UnsupportedRootFSType { typ: String },
	}
}
