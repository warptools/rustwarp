use std::path::PathBuf;

use tokio::task::JoinError;

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

	#[error("failed to parse image manifest: {0}")]
	ParseManifest(serde_json::Error),

	#[error("failed to parse cache index: {0}")]
	ParseCacheIndex(serde_json::Error),

	#[error("failed io operation: {0}")]
	IO(#[from] std::io::Error),

	#[error("failed to chmod: {0}")]
	ChangeMode(#[from] file_mode::ModeError),

	#[error("config: unsupported rootfs.type: {typ}")]
	UnsupportedRootFSType { typ: String },

	#[error("tokio: {0}")]
	TokioError(#[from] JoinError),

	#[error("failed to obtain image cache lock: '{0}'")]
	CacheLockTimeout(PathBuf),
}
