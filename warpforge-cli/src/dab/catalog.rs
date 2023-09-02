use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use warpforge_api::catalog::{CatalogModule, CatalogRelease};
use warpforge_api::catalog::{ModuleName, ReleaseName};

use std::error::Error as UndertypedError;

pub trait Handle {
	fn load_module(
		&self,
		module_name: &ModuleName,
	) -> Result<CatalogModule, Box<dyn UndertypedError>>;
	fn load_release(
		&self,
		module_name: &ModuleName,
		release_name: &ReleaseName,
	) -> Result<CatalogRelease, Box<dyn UndertypedError>>;
}

pub struct FsHandle {
	root_path: PathBuf,
}

impl FsHandle {
	pub fn new<P: AsRef<Path>>(path: P) -> Self {
		Self {
			root_path: path.as_ref().to_path_buf(),
		}
	}
}

impl Handle for FsHandle {
	fn load_module(
		&self,
		module_name: &ModuleName,
	) -> Result<CatalogModule, Box<dyn UndertypedError>> {
		let catmod_index_file_path: PathBuf =
			self.root_path.join(&module_name.0).join("_module.json");
		let reader = BufReader::new(File::open(catmod_index_file_path)?);
		let result = serde_json::from_reader(reader)?;
		// TODO validate the name doesn't conflict with the path we took to get here.
		Ok(result)
	}

	fn load_release(
		&self,
		module_name: &ModuleName,
		release_name: &ReleaseName,
	) -> Result<CatalogRelease, Box<dyn UndertypedError>> {
		let catrel_file_path: PathBuf = self
			.root_path
			.join(&module_name.0)
			.join("_releases")
			.join(release_name.0.clone() + ".json");
		let reader = BufReader::new(File::open(catrel_file_path)?);
		let result = serde_json::from_reader(reader)?;
		// TODO validate the name doesn't conflict with the path we took to get here.
		Ok(result)
	}
}
