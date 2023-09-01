use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use warpforge_api::catalog::CatalogModule;

trait Handle {
	fn load_module(&self, module_name: &warpforge_api::catalog::ModuleName) -> Result<CatalogModule, Box<dyn std::error::Error>>;
}

struct FsHandle {
	root_path: PathBuf,
}

impl FsHandle {
	pub fn new<P: AsRef<Path>>(path: P) -> Self {
		Self { root_path: path.as_ref().to_path_buf() }
	}
}

impl Handle for FsHandle {
	fn load_module(&self, module_name: &warpforge_api::catalog::ModuleName) -> Result<CatalogModule, Box<dyn std::error::Error>> {
		let catmod_index_file_path: PathBuf = self.root_path.join(&module_name.0).join("_module.json");
		let reader = BufReader::new(File::open(catmod_index_file_path)?);
		let result = serde_json::from_reader(reader)?;
		// TODO validate the name doesn't conflict with the path we took to get here.
		Ok(result)
	}
}

fn main() {
	let b = true;
	// This `match` clause is just to demo syntax for how dynamic construction of various Handle implementations can work.
	let instance: Box<dyn Handle> = match b {
		// Both of the below arg styles work seamlessly because of the `P: AsRef<Path>` hijinx.
		true => Box::new(FsHandle::new(Path::new("asdf"))),
		false => Box::new(FsHandle::new("qwer")),
	};

	instance.load_module(&warpforge_api::catalog::ModuleName("hayo".to_string()));
}
