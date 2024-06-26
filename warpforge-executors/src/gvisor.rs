use str_cat::os_str_cat;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use warpforge_terminal::logln;

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

pub struct GvisorExecutor {
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
	pub async fn run(
		&self,
		task: &crate::ContainerParams,
		outbox: tokio::sync::mpsc::Sender<crate::Event>,
	) -> Result<(), crate::Error> {
		let ident: &str = "containernamegoeshere"; // todo: generate this.
		self.prep_bundledir(ident, task)?;
		self.container_exec(ident, task, outbox).await?;
		Ok(())
	}

	fn prep_bundledir(
		&self,
		ident: &str,
		_task: &crate::ContainerParams,
	) -> Result<(), crate::Error> {
		// Build the config data.
		let mut spec = crate::oci::oci_spec_base();
		// todo: apply mutations here.
		let p: json_patch::Patch = serde_json::from_value(serde_json::json!([
			{ "op": "add", "path": "/process/args", "value": ["/bin/bash", "--version"] },
			{ "op": "replace", "path": "/root/path", "value": "/tmp/rootfs" }, // FIXME: time to get the rest of the supply chain implemented :D
			// { "op": "add", "path": "/mounts/-", "value": crate::MountSpec{
			// 		destination: todo!(),
			// 		kind: todo!(),
			// 		source: todo!(),
			// 		options: todo!()
			// 	}.to_oci_mount() },
		]))
		.unwrap();
		json_patch::patch(&mut spec, &p).unwrap();

		// Write it out.
		let cfg_dir = self.ersatz_dir.join(ident);
		fs::create_dir_all(&cfg_dir).map_err(|e| {
			let msg = "failed during executor internals: couldn't create bundle dir".to_owned();
			match e.kind() {
				std::io::ErrorKind::PermissionDenied => crate::Error::SystemSetupError {
					msg,
					cause: Box::new(e),
				},
				_ => crate::Error::SystemRuntimeError {
					msg,
					cause: Box::new(e),
				},
			}
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
			crate::Error::Catchall {
				msg: "unable to serialize OCI spec file".to_owned(),
				cause: Box::new(e),
			}
		})?;
		Ok(())
	}

	async fn container_exec(
		&self,
		ident: &str,
		_task: &crate::ContainerParams,
		outbox: tokio::sync::mpsc::Sender<crate::Event>,
	) -> Result<(), crate::Error> {
		let mut cmd = Command::new("gvisor");
		cmd.arg(os_str_cat!("--debug-log=", self.log_dir));
		cmd.arg("--debug");
		cmd.arg("--strace");
		cmd.arg("--rootless");
		cmd.arg("--network=none"); // must be either this or "host" in gvisor's rootless mode.
		cmd.arg("run");
		cmd.arg(os_str_cat!("--bundle=", self.ersatz_dir.join(ident)));
		cmd.arg(ident); // container name.

		cmd.stdin(Stdio::null());
		cmd.stdout(Stdio::piped());
		cmd.stderr(Stdio::inherit());

		logln!("about to spawn");
		let mut child = cmd.spawn().map_err(|e| {
			let msg = "failed to spawn containerization process".to_owned();
			match e.kind() {
				std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
					crate::Error::SystemSetupError {
						msg,
						cause: Box::new(e),
					}
				}
				_ => crate::Error::SystemRuntimeError {
					msg,
					cause: Box::new(e),
				},
			}
		})?;
		println!("somehow, spawned");

		// Take handles to the IO before we spawn the exit wait.
		// (The exit wait future takes ownership of the `child` value.)
		let stdout = child
			.stdout
			.take()
			.expect("child did not have a handle to stdout");

		let childwait_handle = tokio::spawn(async move {
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
			outbox
				.send(crate::Event {
					topic: ident.to_owned(),
					body: crate::events::EventBody::Output {
						channel: 1,
						val: line,
					},
				})
				.await
				.expect("channel must not be closed");
		}

		childwait_handle.await.map_err(|e| crate::Error::Catchall {
			msg: "error from child process".to_owned(),
			cause: Box::new(e),
		})
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use indexmap::IndexMap;
	use tokio::sync::mpsc;

	#[tokio::test]
	#[ignore = "deprecated"]
	async fn gvisor_it_works() {
		let cfg = super::GvisorExecutor {
			ersatz_dir: Path::new("/tmp/warpforge-test-executor-gvisor/run").to_owned(),
			log_dir: Path::new("/tmp/warpforge-test-executor-gvisor/log").to_owned(),
		};
		let (gather_chan, mut gather_chan_recv) = mpsc::channel(32);
		let params = crate::ContainerParams {
			command: vec!["/bin/sh -c \"echo it works!\"".to_string()],
			mounts: {
				// IndexMap does have a From trait, but I didn't want to copy the destinations manually.
				IndexMap::new()
				// todo: more initializer here
			},
			environment: IndexMap::new(),
			root_path: "/tmp/rustwarp-rootfs/".to_string(),
		};
		// We let this greenthread sail off into the dark.
		tokio::spawn(async move {
			while let Some(evt) = gather_chan_recv.recv().await {
				println!("event! {:?}", evt)
			}
		});
		cfg.run(&params, gather_chan).await.expect("it didn't fail");
	}
}
