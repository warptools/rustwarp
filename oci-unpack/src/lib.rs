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

pub(crate) mod tee;

use std::{
	fs::{self},
	io::Read,
	path::Path,
	time::UNIX_EPOCH,
};

use file_mode::ModePath;
use filetime::set_file_times;
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
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::tee::ReadExt;

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

	// From opencontainers/umoci:
	// "We change the mode of the bundle directory to 0700. A user can easily
	// change this after-the-fact, but we do this explicitly to avoid cases
	// where an unprivileged user could recurse into an otherwise unsafe image
	// (giving them potential root access through setuid binaries for example)."
	target.set_mode(0o700)?;

	let image_data = pull_image(image, auth).await?;

	let rootfs_dir = target.as_ref().join("rootfs");
	fs::create_dir(&rootfs_dir)?;
	// TODO: root UID and root GID mapping.

	// From opencontainers/umoci:
	// "Currently, many different images in the wild don't specify what the
	// atime/mtime of the root directory is. This is a huge pain because it
	// means that we can't ensure consistent unpacking. In order to get around
	// this, we first set the mtime of the root directory to the Unix epoch
	// (which is as good of an arbitrary choice as any)."
	set_file_times(&target, UNIX_EPOCH.into(), UNIX_EPOCH.into())?;

	let rootfs_config = image_data.config.rootfs();
	if rootfs_config.typ() != "layers" {
		let typ = rootfs_config.typ().to_string();
		return Err(Error::UnsupportedRootFSType { typ });
	}

	let diff_ids = rootfs_config.diff_ids();
	if image_data.layers.len() != diff_ids.len() {
		let reason = "len(layers) != len(diff_ids)".to_string();
		return Err(Error::ImageInvalid(reason));
	}

	for (layer, diff_id) in image_data.layers.iter().zip(diff_ids) {
		unpack_layer(layer, diff_id, &rootfs_dir)?;
	}

	// TODO: Should we unpack a config.json here or do we create
	// that from scratch using the warpforge build instructions?

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

fn unpack_layer(layer: &ImageLayer, diff_id: &str, target: impl AsRef<Path>) -> Result<()> {
	if is_gzip(&layer.media_type) {
		unpack_layer_gzip(&layer.data[..], diff_id, target)
	} else {
		unpack_layer_tar(&layer.data[..], diff_id, target)
	}
}

fn unpack_layer_gzip(data: impl Read, diff_id: &str, target: impl AsRef<Path>) -> Result<()> {
	let decoder = GzDecoder::new(data);
	unpack_layer_tar(decoder, diff_id, target)
}

fn unpack_layer_tar(data: impl Read, diff_id: &str, target: impl AsRef<Path>) -> Result<()> {
	if !diff_id.starts_with("sha256:") {
		let algorithm = diff_id.split(':').next().unwrap_or("none");
		let reason = format!("unsupported digest algorithm: {0}", algorithm);
		return Err(Error::UnsupportedFeature(reason));
	}
	let mut digester = Sha256::new();

	let mut read = data.tee(&mut digester);
	tar::Archive::new(&mut read).unpack(&target)?;

	// From opencontainers/umoci:
	// "Different tar implementations can have different levels of redundant
	// padding and other similar weird behaviours. While on paper they are
	// all entirely valid archives [...]. Just blindly consume anything left
	// in the layer."
	let mut buffer = Vec::new();
	let length = read.read_to_end(&mut buffer)?;

	let diff_id = &diff_id["sha256:".len()..];
	let computed = format!("{:x}", digester.finalize());
	if diff_id != computed {
		if length > 0 {
			eprintln!("oci-unpack: ignored {length} trailing 'junk' bytes in the tar stream -- probably from GNU tar")
		}
		return Err(Error::LayerDiffIdMismatch);
	}

	Ok(())
}

pub mod error {
	pub type Result<T> = std::result::Result<T, Error>;

	#[derive(thiserror::Error, Debug)]
	pub enum Error {
		#[error("target directory was not empty")]
		TargetNotEmpty,

		#[error("failed to download image: {0}")]
		DownloadFailed(#[from] oci_client::errors::OciDistributionError),

		#[error("invalid image: {0}")]
		ImageInvalid(String),

		#[error("feature not supported: {0}")]
		UnsupportedFeature(String),

		#[error("layer tar diff_id mismatch")]
		LayerDiffIdMismatch,

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
