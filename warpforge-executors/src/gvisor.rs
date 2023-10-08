use str_cat::os_str_cat;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

struct GvisorExecutor {
	/// Path to use for:
	///   - the generated short-lived container spec files
	///   - the generated rootfs dir (into which mounts are landed!)
	///   - the upperdir and workdir for any generated overlayfs mounts
	///       (note: these can be picky about what the host filesystem is!)
	///
	/// FIXME: I'm still wobbling on whether we want a level of executor API that doesn't understand formulas directly.
	/// If we're going that way, then this comment about upperdir and workdir is probably a lie.
	/// But really, the rate at which I want to intentionally specify those is, uh.  Approximately never.
	pub ersatz_dir: PathBuf,

	/// Path used to store logs.
	/// We'll create a whole directory in this per invocation, because gvisor creates many log files per container.
	pub log_dir: PathBuf,
}

impl GvisorExecutor {
	pub async fn run(&self, task: &crate::ContainerParams) -> Result<(), crate::Error> {
		let ident: &str = "containernamegoeshere"; // todo: generate this.
		self.prep_bundledir(ident, task)?;
		self.container_exec(ident, task).await?;
		Ok(())
	}

	fn prep_bundledir(
		&self,
		ident: &str,
		task: &crate::ContainerParams,
	) -> Result<(), crate::Error> {
		// Build the config data.
		let mut spec = crate::oci::oci_spec_base();
		// todo: apply mutations here.

		// Write it out.
		let cfg_dir = self.ersatz_dir.join(ident);
		fs::create_dir_all(&cfg_dir).map_err(|e| crate::Error::Catchall {
			msg: "failed during executor internals: couldn't create bundle dir".to_owned(),
			cause: Box::new(e),
		})?;
		let f = fs::File::create(cfg_dir.join("config.json")) // Must literally be this name within bundle dir.
			.map_err(|e| crate::Error::Catchall {
				msg:
					"failed during executor internals: couldn't open bundle config file for writing"
						.to_owned(),
				cause: Box::new(e),
			})?;
		serde_json::to_writer(f, &spec).map_err(|e| {
			if e.is_io() {
				return crate::Error::Catchall {
					msg: "failed during executor internals: io error writing config file"
						.to_owned(),
					cause: Box::new(Into::<std::io::Error>::into(e)),
				};
			}
			return crate::Error::Catchall {
				msg: "unable to serialize OCI spec file".to_owned(),
				cause: Box::new(e),
			};
		})?;
		Ok(())
	}

	async fn container_exec(
		&self,
		ident: &str,
		task: &crate::ContainerParams,
	) -> Result<(), crate::Error> {
		let mut cmd = Command::new("gvisor");
		cmd.args([os_str_cat!("--debug-log=", self.log_dir)]);
		cmd.args(["--debug", "--strace"]);
		cmd.args(["--rootless"]);
		cmd.args(["--network=none"]); // must be either this or "host" in gvisor's rootless mode.
		cmd.args(["run"]);
		cmd.args([os_str_cat!("--bundle=", self.ersatz_dir.join(ident))]);
		cmd.args([ident]); // container name.

		cmd.stdin(Stdio::null());
		cmd.stdout(Stdio::piped());
		cmd.stderr(Stdio::inherit());

		println!("about to spawn");
		let mut child = cmd.spawn().expect("failed to spawn command");
		println!("somehow, spawned");

		// Take handles to the IO before we spawn the exit wait.
		// (The exit wait future takes ownership of the `child` value.)
		let stdout = child
			.stdout
			.take()
			.expect("child did not have a handle to stdout");

		tokio::spawn(async move {
			let status = child
				.wait()
				.await
				.expect("child process encountered an error");
			// FIXME errors need to go to a channel.
			println!("child status was: {}", status);
		});

		let mut reader = tokio::io::BufReader::new(stdout).lines();

		while let Some(line) = reader
			.next_line()
			.await
			.map_err(|e| crate::Error::Catchall {
				msg: "system io error communicating with subprocess during executor run".to_owned(),
				cause: Box::new(e),
			})? {
			println!("relayed: {}", line);
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use indexmap::IndexMap;

	#[tokio::main]
	#[test]
	async fn it_works() {
		let cfg = super::GvisorExecutor {
			ersatz_dir: Path::new("/tmp/warpforge-test-executor-gvisor/run").to_owned(),
			log_dir: Path::new("/tmp/warpforge-test-executor-gvisor/log").to_owned(),
		};
		let params = crate::ContainerParams {
			mounts: (|| {
				// IndexMap does have a From trait, but I didn't want to copy the destinations manually.
				IndexMap::new()
				// todo: more initializer here
			})(),
		};
		let res = cfg.run(&params).await.expect("it didn't fail");
	}
}
