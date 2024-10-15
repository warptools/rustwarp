use std::io::{BufRead, BufReader, Lines};
use std::path::PathBuf;
use std::process::Stdio;
use std::thread;
use std::{fs, process::Command};

use crossbeam_channel::Sender;
use str_cat::os_str_cat;

use crate::{Error, Result};

pub struct Executor {
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

	/// File to write logs.
	pub log_file: PathBuf,
}

impl Executor {
	pub fn run(&self, task: &crate::ContainerParams, outbox: Sender<crate::Event>) -> Result<()> {
		self.prep_bundledir(task)?;
		self.container_exec(task, outbox)?;
		Ok(())
	}

	fn prep_bundledir(&self, task: &crate::ContainerParams) -> Result<()> {
		// Build the config data.
		let mut spec = crate::oci::oci_spec_base();

		use syscalls::{syscall, Sysno};
		let uid = match unsafe { syscall!(Sysno::getuid) } {
			Ok(uid) => uid,
			Err(err) => {
				eprintln!("syscall getuid() failed: {}", err);
				0
			}
		};
		let gid = match unsafe { syscall!(Sysno::getgid) } {
			Ok(id) => id,
			Err(err) => {
				eprintln!("syscall getgid() failed: {}", err);
				0
			}
		};

		// todo: apply mutations here.
		let p: json_patch::Patch = serde_json::from_value(serde_json::json!([
			{ "op": "add", "path": "/process/args", "value": task.command },
			{ "op": "replace", "path": "/root/path", "value": task.root_path }, // FIXME: time to get the rest of the supply chain implemented :D
			{ "op": "add", "path": "/linux/uidMappings", "value":
			   [{"containerID": 0, "hostID": uid, "size": 1}]},
			{ "op": "add", "path": "/linux/gidMappings", "value":
			   [{"containerID": 0, "hostID": gid, "size": 1}]},
			{ "op": "add", "path": "/linux/namespaces/-", "value": {"type": "user"}},
		]))
		.unwrap();
		json_patch::patch(&mut spec, &p).unwrap();

		// add mount specs
		use crate::oci::ToOCIMount;
		for (_dest, ms) in task.mounts.iter() {
			let p: json_patch::Patch = serde_json::from_value(serde_json::json!([
				{ "op": "add", "path": "/mounts/-", "value": ms.to_oci_mount() },
			]))
			.unwrap();
			json_patch::patch(&mut spec, &p).unwrap();
		}

		// add environment variables
		for (var, val) in task.environment.iter() {
			let p: json_patch::Patch = serde_json::from_value(serde_json::json!([
				{ "op": "add", "path": "/process/env/-", "value": format!("{var}={val}")}
			]))
			.unwrap();
			json_patch::patch(&mut spec, &p).unwrap();
		}

		// Write it out.
		let cfg_dir = self.ersatz_dir.join(&task.ident);
		fs::create_dir_all(&cfg_dir).map_err(|e| {
			let msg = "failed during executor internals: couldn't create bundle dir".to_owned();
			match e.kind() {
				std::io::ErrorKind::PermissionDenied => Error::SystemSetupError {
					msg,
					cause: Box::new(e),
				},
				_ => Error::SystemRuntimeError {
					msg,
					cause: Box::new(e),
				},
			}
		})?;
		let f = fs::File::create(cfg_dir.join("config.json")) // Must literally be this name within bundle dir.
			.map_err(|e| Error::Catchall {
				msg:
					"failed during executor internals: couldn't open bundle config file for writing"
						.to_owned(),
				cause: Box::new(e),
			})?;
		serde_json::to_writer_pretty(f, &spec).map_err(|e| {
			if e.is_io() {
				return Error::Catchall {
					msg: "failed during executor internals: io error writing config file"
						.to_owned(),
					cause: Box::new(Into::<std::io::Error>::into(e)),
				};
			}
			Error::Catchall {
				msg: "unable to serialize OCI spec file".to_owned(),
				cause: Box::new(e),
			}
		})?;
		Ok(())
	}

	fn container_exec(
		&self,
		task: &crate::ContainerParams,
		outbox: Sender<crate::Event>,
	) -> Result<()> {
		let mut cmd = Command::new(&task.runtime);
		cmd.arg(os_str_cat!("--log=", self.log_file));
		cmd.arg("--debug");
		cmd.arg("run");
		cmd.arg(os_str_cat!("--bundle=", self.ersatz_dir.join(&task.ident)));
		cmd.arg(&task.ident); // container name.

		cmd.stdin(Stdio::null());
		cmd.stdout(Stdio::piped());
		cmd.stderr(Stdio::piped());

		let mut child = cmd.spawn().map_err(|e| {
			let msg = "failed to spawn containerization process".to_owned();
			match e.kind() {
				std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
					Error::SystemSetupError {
						msg,
						cause: Box::new(e),
					}
				}
				_ => Error::SystemRuntimeError {
					msg,
					cause: Box::new(e),
				},
			}
		})?;

		// Take handles to the IO before we spawn the exit wait.
		// (The exit wait future takes ownership of the `child` value.)
		let stdout = BufReader::new(
			child
				.stdout
				.take()
				.expect("child did not have a handle to stdout"),
		);
		let stderr = BufReader::new(
			child
				.stderr
				.take()
				.expect("child did not have a handle to stderr"),
		);

		let handle = {
			let outbox = outbox.clone();
			let task_ident = task.ident.to_owned();
			thread::spawn::<_, std::io::Result<()>>(move || {
				Self::send_container_output(&task_ident, &outbox, 2, stderr.lines())
			})
		};

		Self::send_container_output(&task.ident, &outbox, 1, stdout.lines()).map_err(|e| {
			Error::Catchall {
				msg: "failed to read stdout from container".to_owned(),
				cause: Box::new(e),
			}
		})?;

		handle.join().unwrap().map_err(|e| Error::Catchall {
			msg: "failed to read stderr from container".to_owned(),
			cause: Box::new(e),
		})?;

		let status = child.wait().map_err(|err| Error::SystemRuntimeError {
			msg: "failed to get child exit code".into(),
			cause: Box::new(err),
		})?;

		outbox
			.send(crate::Event {
				topic: task.ident.to_owned(),
				body: crate::events::EventBody::ExitCode(status.code()),
			})
			.expect("channel must not be closed");

		Ok(())
	}

	fn send_container_output<T: BufRead>(
		ident: &str,
		outbox: &Sender<crate::Event>,
		channel: i32,
		lines: Lines<T>,
	) -> std::io::Result<()> {
		for line in lines {
			let line = line?;
			outbox
				.send(crate::Event {
					topic: ident.to_owned(),
					body: crate::events::EventBody::Output { channel, val: line },
				})
				.expect("channel must not be closed");
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{path::PathBuf, thread};

	use indexmap::IndexMap;
	use oci_unpack::{pull_and_unpack, PullConfig};
	use tempfile::TempDir;

	use crate::events::EventBody;

	#[test]
	fn execute_it_works() {
		let temp_dir = TempDir::new().unwrap();
		let path = temp_dir.path();

		let image = &"docker.io/busybox:latest".parse().unwrap();
		let bundle_path = path.join("bundle");
		let pull_config = PullConfig {
			cache: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.images")),
			..Default::default()
		};
		pull_and_unpack(image, &bundle_path, &pull_config).unwrap();

		let cfg = crate::execute::Executor {
			ersatz_dir: path.join("run"),
			log_file: path.join("log"),
		};
		let (gather_chan, gather_chan_recv) = crossbeam_channel::bounded::<crate::Event>(32);
		let params = crate::ContainerParams {
			ident: "containernamegoeshere".into(),
			runtime: "runc".into(),
			command: vec![
				"/bin/sh".to_string(),
				"-c".to_string(),
				"echo $MSG".to_string(),
			],
			mounts: { IndexMap::new() },
			root_path: bundle_path.join("rootfs"),

			environment: IndexMap::from([
				("MSG".into(), "hello, from environment variables!".into()),
				("VAR".into(), "test".into()),
			]),
		};

		let gather_handle = thread::spawn(move || {
			while let Ok(evt) = gather_chan_recv.recv() {
				match &evt.body {
					EventBody::Output { val, channel: 1 } => println!("[container] {val}"),
					EventBody::Output { val, channel: 2 } => eprintln!("[container] {val}"),
					EventBody::Output { .. } => panic!("invalid channel number"),
					EventBody::ExitCode(code) => {
						assert_eq!(code, &Some(0));
						break; // stop processing events
					}
				};
			}
		});

		cfg.run(&params, gather_chan).expect("it didn't fail");
		gather_handle.join().expect("gathering events failed");
	}
}
