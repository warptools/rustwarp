// TODO: Function to remove image from cache.
// TODO: GC function to remove orpahn blobs.

use std::{
	fs::{self, OpenOptions},
	io::{ErrorKind, Write},
	path::{Path, PathBuf},
	thread::sleep,
	time::{Duration, Instant},
};

use indexmap::{map, IndexMap};
use oci_client::{
	client::{ImageData, ImageLayer},
	manifest::{OciImageManifest, OCI_IMAGE_MEDIA_TYPE},
	Reference,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, PullConfig, Result};

const BLOBS_DIR: &str = "blobs";
const INDEX_FILE: &str = "index.json";
const LOCK_FILE: &str = "index.lock";

const CACHE_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct Cache<'a> {
	config: &'a PullConfig,
	has_lock: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Index {
	images: IndexMap<String, IndexImage>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct IndexImage {
	manifest_digest: String,
}

impl<'a> Drop for Cache<'a> {
	fn drop(&mut self) {
		if self.has_lock {
			let path = self.config.cache.as_ref().unwrap().join(LOCK_FILE);
			fs::remove_file(path).expect("failed to remove cache lock file");
		}
	}
}

impl<'a> Cache<'a> {
	pub(crate) fn new(config: &'a PullConfig) -> Self {
		Cache {
			config,
			has_lock: false,
		}
	}

	pub(crate) fn before_pull(&mut self, image: &Reference) -> Result<Option<ImageData>> {
		let Some(cache_dir) = &self.config.cache else {
			return Ok(None);
		};

		if !cache_dir.is_dir() {
			init_cache(cache_dir)?;
		}

		match image_from_reference(cache_dir, image) {
			Ok(None) | Err(Error::ParseCacheIndex(_)) => {}
			result @ (Ok(Some(_)) | Err(_)) => return result,
		}

		// Trying to lock index. We do this, because we are going to report
		// a cache miss and therefore trigger a pull from the registry next.
		// To prevent redundant downloads (e.g. during testing), we lock here.
		self.lock_index(cache_dir)?;

		// Prevent race condition: check again if we do not have image by now.
		if let Some(image_data) = image_from_reference(cache_dir, image)? {
			return Ok(Some(image_data));
		}

		Ok(None)
	}

	pub(crate) async fn after_pull(
		&mut self,
		image: &Reference,
		image_data: &ImageData,
		client: &oci_client::Client,
		config: &PullConfig,
	) -> Result<()> {
		let Some(cache_dir) = &config.cache else {
			return Ok(());
		};
		assert!(self.has_lock, "should have called before_pull");

		// Obtain exact manifest bytes, because we need to make sure digest of manifest is correct.
		let digest = image_data.digest.as_ref().unwrap();
		let media_type = &image_data.manifest.as_ref().unwrap().media_type;
		let media_type = media_type.as_ref().map(AsRef::as_ref);
		let media_type = [media_type.unwrap_or(OCI_IMAGE_MEDIA_TYPE)];
		let reference = image.clone_with_digest(digest.to_owned());
		let (manifest_raw, digest_raw) = client
			.pull_manifest_raw(&reference, &config.auth, &media_type)
			.await?;
		assert_eq!(digest, &digest_raw);

		add_image_to_cache(cache_dir, image, image_data, &manifest_raw)
	}

	fn lock_index(&mut self, cache_dir: impl AsRef<Path>) -> Result<()> {
		if self.has_lock {
			return Ok(());
		}

		let path = cache_dir.as_ref().join(LOCK_FILE);
		let start = Instant::now();

		loop {
			match OpenOptions::new().write(true).create_new(true).open(&path) {
				Ok(_) => {
					self.has_lock = true;
					return Ok(());
				}
				Err(error) if error.kind() != ErrorKind::AlreadyExists => return Err(error.into()),
				_ => {
					// Lock file already exists: wait and try again.
					// TODO: Use notify instead of polling repeatedly.
					while path.exists() {
						if start.elapsed() >= CACHE_LOCK_TIMEOUT {
							return Err(Error::CacheLockTimeout(path));
						}
						sleep(Duration::from_millis(50));
					}
				}
			}
		}
	}
}

fn get_index(cache_dir: impl AsRef<Path>) -> Result<Index> {
	fn get_index_raw(cache_dir: impl AsRef<Path>) -> Result<Index> {
		let contents = fs::read(cache_dir.as_ref().join(INDEX_FILE))?;
		serde_json::from_slice(&contents[..]).map_err(Error::ParseCacheIndex)
	}

	match get_index_raw(&cache_dir) {
		Ok(index) => Ok(index),
		Err(_) => {
			sleep(Duration::from_millis(100));
			get_index_raw(&cache_dir) // retry
		}
	}
}

fn init_cache(cache_dir: impl AsRef<Path>) -> Result<()> {
	fs::create_dir_all(cache_dir.as_ref().join(BLOBS_DIR))?;

	let file =
		(OpenOptions::new().write(true).create_new(true)).open(cache_dir.as_ref().join(INDEX_FILE));
	match file {
		Ok(mut file) => {
			let index = Index {
				images: IndexMap::with_capacity(0),
			};
			let data = serde_json::to_vec(&index).map_err(Error::ParseCacheIndex)?;
			file.write_all(&data[..])?;
		}
		Err(error) if error.kind() != ErrorKind::AlreadyExists => return Err(error.into()),
		_ => {}
	}

	Ok(())
}

fn image_from_reference(
	cache_dir: impl AsRef<Path>,
	image: &Reference,
) -> Result<Option<ImageData>> {
	let mut index = get_index(&cache_dir)?;
	if let map::Entry::Occupied(entry) = index.images.entry(image.whole()) {
		return Ok(Some(image_from_blobs(
			cache_dir,
			&entry.get().manifest_digest,
		)?));
	}
	Ok(None)
}

fn image_from_blobs(cache_dir: impl AsRef<Path>, manifest_digest: &str) -> Result<ImageData> {
	let blob_dir = cache_dir.as_ref().join(BLOBS_DIR);

	let manifest_data = read_blob(&blob_dir, manifest_digest)?;
	let manifest: OciImageManifest =
		serde_json::from_slice(&manifest_data[..]).map_err(Error::ParseManifest)?;

	let config_digest = &manifest.config.digest;
	let config_data = read_blob(&blob_dir, config_digest)?;
	let config = oci_client::client::Config {
		data: config_data,
		media_type: manifest.config.media_type.to_owned(),
		annotations: manifest.config.annotations.clone(),
	};

	let mut layers = Vec::new();
	for layer in &manifest.layers {
		let data = read_blob(&blob_dir, &layer.digest)?;
		layers.push(ImageLayer {
			data,
			media_type: layer.media_type.clone(),
			annotations: layer.annotations.clone(),
		});
	}

	Ok(ImageData {
		manifest: Some(manifest),
		digest: Some(manifest_digest.to_owned()),
		config,
		layers,
	})
}

fn add_image_to_cache(
	cache_dir: impl AsRef<Path>,
	image: &Reference,
	image_data: &ImageData,
	manifest_raw: &[u8],
) -> Result<()> {
	let blob_dir = cache_dir.as_ref().join(BLOBS_DIR);

	let digest = image_data.digest.as_ref().unwrap();
	add_blob_to_cache(&blob_dir, digest, manifest_raw)?;

	let config_digest = &image_data.manifest.as_ref().unwrap().config.digest;
	add_blob_to_cache(&blob_dir, config_digest, &image_data.config.data[..])?;

	let manifest_layers = &image_data.manifest.as_ref().unwrap().layers;
	assert_eq!(manifest_layers.len(), image_data.layers.len());
	for (i, layer) in image_data.layers.iter().enumerate() {
		add_blob_to_cache(&blob_dir, &manifest_layers[i].digest, &layer.data[..])?;
	}

	let index_path = cache_dir.as_ref().join(INDEX_FILE);
	let mut index = get_index(&cache_dir)?;
	index.images.insert(
		image.whole(),
		IndexImage {
			manifest_digest: digest.to_owned(),
		},
	);
	let index_bytes = serde_json::to_vec(&index).unwrap();
	fs::write(index_path, &index_bytes[..])?;

	Ok(())
}

fn add_blob_to_cache(
	blob_dir: impl AsRef<Path>,
	digest: &str,
	data: impl AsRef<[u8]>,
) -> Result<()> {
	if !digest.starts_with("sha256:") {
		return Err(Error::DigestNotSupported {
			digest: digest.to_owned(),
		});
	}

	let colon = digest.find(':').unwrap();
	let blob_path = blob_path(blob_dir, digest, colon);
	fs::create_dir_all(&blob_path)?;
	Ok(fs::write(blob_path.join(&digest[colon + 1..]), data)?)
}

fn read_blob(blob_dir: impl AsRef<Path>, digest: &str) -> Result<Vec<u8>> {
	let colon = digest.find(':').unwrap();
	let data = fs::read(blob_path(blob_dir, digest, colon).join(&digest[colon + 1..]))?;

	let actual = format!("sha256:{:x}", Sha256::digest(&data));
	if actual != digest {
		return Err(Error::CorruptCacheBlob {
			digest: digest.to_owned(),
		});
	}

	Ok(data)
}

fn blob_path(blob_dir: impl AsRef<Path>, digest: &str, colon: usize) -> PathBuf {
	// TODO: check that `rest.len() >= 4`
	let rest = &digest[colon + 1..];
	blob_dir
		.as_ref()
		.join(&digest[..colon])
		.join(&rest[..2])
		.join(&rest[2..4])
}
