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

mod cache;
mod config;
mod error;
pub mod tee;

use std::{
	fs::{self},
	io::Read,
	path::Path,
	time::UNIX_EPOCH,
};

use cache::Cache;
use file_mode::ModePath;
use filetime::set_file_times;
use flate2::read::GzDecoder;
use oci_client::{
	client::{ClientConfig, ImageLayer},
	manifest::{
		OciImageManifest, IMAGE_DOCKER_LAYER_GZIP_MEDIA_TYPE, IMAGE_DOCKER_LAYER_TAR_MEDIA_TYPE,
		IMAGE_LAYER_GZIP_MEDIA_TYPE, IMAGE_LAYER_MEDIA_TYPE, IMAGE_MANIFEST_LIST_MEDIA_TYPE,
		IMAGE_MANIFEST_MEDIA_TYPE, OCI_IMAGE_INDEX_MEDIA_TYPE, OCI_IMAGE_MEDIA_TYPE,
	},
	Client, Reference,
};
use oci_spec::image::ImageConfiguration;
use sha2::{Digest, Sha256};

pub use crate::config::PullConfig;
pub use crate::error::{Error, Result};
use crate::tee::ReadExt;

const MANIFEST_MEDIA_TYPES: &[&str] = &[
	IMAGE_MANIFEST_MEDIA_TYPE,
	IMAGE_MANIFEST_LIST_MEDIA_TYPE,
	OCI_IMAGE_MEDIA_TYPE,
	OCI_IMAGE_INDEX_MEDIA_TYPE,
];

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

pub fn pull_and_unpack(
	image: &Reference,
	target: impl AsRef<Path>,
	config: &PullConfig,
) -> Result<BundleInfo> {
	let image_data = pull_image(image, config)?;

	let manifest = image_data.manifest.unwrap();
	let manifest_digest = image_data.digest.unwrap();
	let config: ImageConfiguration =
		serde_json::from_slice(&image_data.config.data).map_err(Error::ParseImageConfiguration)?;

	let image_data = ImageData {
		manifest,
		manifest_digest,
		layers: image_data.layers,
		config,
	};

	unpack(target, image_data)
}

pub fn unpack(
	target: impl AsRef<Path>,
	image_data: ImageData,
) -> std::result::Result<BundleInfo, Error> {
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

pub struct ImageData {
	manifest: OciImageManifest,
	manifest_digest: String,
	layers: Vec<ImageLayer>,
	config: ImageConfiguration,
}

#[tokio::main]
pub async fn pull_image(
	image: &Reference,
	config: &PullConfig,
) -> Result<oci_client::client::ImageData> {
	let mut cache = Cache::new(config);
	if let Some(image_data) = cache.before_pull(image)? {
		return Ok(image_data);
	}

	let client = Client::new(ClientConfig::default());
	let media_types = LAYER_MEDIA_TYPES.to_vec();

	let image_data = client.pull(image, &config.auth, media_types).await?;

	cache
		.after_pull(image, &image_data, &client, config)
		.await?;

	Ok(image_data)
}

#[tokio::main]
pub async fn pull_image_manifest(image: &Reference, config: &PullConfig) -> Result<String> {
	let mut cache = Cache::new(config);
	if let Some(image_data) = cache.before_pull(image)? {
		return Ok(image_data.digest.unwrap());
	}

	let client = Client::new(ClientConfig::default());
	let manifest = client
		.pull_manifest_raw(image, &config.auth, MANIFEST_MEDIA_TYPES)
		.await?;
	Ok(manifest.1)
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
