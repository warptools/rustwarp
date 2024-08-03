use std::path::PathBuf;

use oci_client::secrets::RegistryAuth;
use oci_unpack::{error::Result, unpack};

#[tokio::main]
async fn main() -> Result<()> {
	let reference = "docker.io/library/busybox:latest".parse().unwrap();
	let auth = RegistryAuth::Anonymous;
	let target: PathBuf = "busybox_bundle".into();

	// TODO: Remove this:
	if target.exists() {
		std::fs::remove_dir_all(&target)?;
	}

	let info = unpack(&reference, &auth, &target).await?;
	println!("{:#?}", info.manifest);

	Ok(())
}
