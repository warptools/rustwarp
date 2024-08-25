use std::path::PathBuf;

use oci_unpack::{pull_and_unpack, PullConfig, Result};

#[tokio::main]
async fn main() -> Result<()> {
	let reference = "docker.io/library/busybox:latest".parse().unwrap();
	let config = PullConfig::default();
	let target: PathBuf = "busybox_bundle".into();

	let info = pull_and_unpack(&reference, &target, &config).await?;
	println!("{:#?}", info.manifest);

	Ok(())
}
