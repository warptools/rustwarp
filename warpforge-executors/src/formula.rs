use indexmap::IndexMap;
use oci_client::secrets::RegistryAuth;
use oci_unpack::unpack;
use rand::distributions::{Alphanumeric, DistString};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tokio::sync::mpsc;
use warpforge_api::formula::{self, ActionScript, FormulaAndContext, FormulaInput};
use warpforge_terminal::logln;

use crate::events::EventBody;
use crate::execute::Executor;
use crate::{ContainerParams, Error, Event, MountSpec, Result};

pub struct Formula {
	executor: Executor,
}

pub async fn run_formula(formula: FormulaAndContext, runtime: PathBuf) -> Result<()> {
	let temporary_dir = tempfile::tempdir().map_err(|err| Error::SystemSetupError {
		msg: "failed to setup temporary dir".into(),
		cause: Box::new(err),
	})?;

	let executor = Formula {
		executor: Executor {
			ersatz_dir: temporary_dir.path().join("run"),
			log_file: temporary_dir.path().join("log"), // TODO: Find a better more persistent location for logs.
		},
	};

	let (event_sender, mut event_receiver) = mpsc::channel::<Event>(32);

	let event_handler = tokio::spawn(async move {
		while let Some(event) = event_receiver.recv().await {
			match &event.body {
				EventBody::Output { val, .. } => logln!("[container] {val}\n"),
				EventBody::ExitCode(code) => return *code,
			}
		}

		None
	});

	executor.run(formula, runtime, event_sender).await?;

	let exit_code = event_handler.await.map_err(|e| Error::SystemRuntimeError {
		msg: "unexpected error while running container".into(),
		cause: Box::new(e),
	})?;
	match exit_code {
		Some(0) => Ok(()),
		_ => Err(Error::SystemRuntimeError {
			msg: "container terminated non-zero exit code".into(),
			cause: exit_code.map_or_else(|| "None".into(), |code| format!("{code}").into()),
		}),
	}
}

impl Formula {
	const CONTAINER_BASE_PATH: &'static str = "/.warpforge.container";

	pub fn container_script_path() -> PathBuf {
		PathBuf::from(Self::CONTAINER_BASE_PATH).join("script")
	}

	pub fn setup_script(
		&self,
		script: &ActionScript,
		mounts: &mut IndexMap<String, MountSpec>,
	) -> Result<Vec<String>> {
		let script_dir = self.executor.ersatz_dir.join("script");

		// We don't want to give the container access to a pre-existing directory.
		if script_dir.exists() {
			let msg = "script directory already existed when trying to setup script".into();
			return Err(Error::SystemSetupCauseless { msg });
		}

		fs::create_dir_all(&script_dir).map_err(|e| {
			let msg = "failed during formula execution: couldn't create script dir".to_owned();
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

		let mut script_file =
			fs::File::create(script_dir.join("run")).map_err(|e| Error::Catchall {
				msg: "failed during formula execution: couldn't open script file for writing"
					.to_owned(),
				cause: Box::new(e),
			})?;

		for (n, line) in script.contents.iter().enumerate() {
			let entry_file_name = format!("entry-{}", n);
			let mut entry_file =
				fs::File::create(script_dir.join(&entry_file_name)).map_err(|e| {
					Error::Catchall {
						msg: "failed during formula execution: couldn't cr".to_owned()
							+ &format!("eate script entry number {}", n).to_string(),
						cause: Box::new(e),
					}
				})?;
			writeln!(entry_file, "{}", line).map_err(|e| Error::Catchall {
				msg: format!(
					"failed during formula execution: io error writing script entry file {n}",
				),
				cause: Box::new(e),
			})?;
			writeln!(
				script_file,
				". {}",
				Self::container_script_path()
					.join(entry_file_name)
					.display()
			)
			.map_err(|e| Error::Catchall {
				msg: format!(
					"failed during formula execution: io error writing script file entry {n}"
				),
				cause: Box::new(e),
			})?;
		}

		// mount the script into the container
		mounts.insert(
			Self::container_script_path().to_str().unwrap().into(),
			MountSpec::new_bind(&script_dir, Self::container_script_path(), false),
		);

		Ok(vec![
			script.interpreter.to_owned(),
			Self::container_script_path()
				.join("run")
				.to_str()
				.unwrap()
				.to_string(),
		])
	}

	pub async fn run(
		&self,
		formula_and_context: warpforge_api::formula::FormulaAndContext,
		runtime: PathBuf,
		outbox: tokio::sync::mpsc::Sender<Event>,
	) -> Result<()> {
		let mut mounts = IndexMap::new();
		let mut environment = IndexMap::new();
		let formula::FormulaCapsule::V1(formula) = formula_and_context.formula;

		// Handle Inputs
		for (formula::SandboxPort(port), input) in formula.inputs {
			//TODO implement the FormulaInputComplex filter thing
			match port.get(..1) {
				// TODO replace this with a catverter macro
				Some("$") => {
					let env_name = match port.get(1..) {
						// Have to check for empty string, because: `"$".get(1..) == Some("")`
						Some(name) if !name.is_empty() => name.to_string(),
						_ => {
							let msg = "environment variable with empty name".into();
							return Err(Error::SystemSetupCauseless { msg });
						}
					};
					let FormulaInput::Literal(env_value) = input else {
						let msg =
							format!("value of environment variable '{env_name}' has to be literal");
						return Err(Error::SystemSetupCauseless { msg });
					};

					environment.insert(env_name, env_value);
				}
				Some("/") => {}
				_ => {
					let msg = format!("invalid formula input '{}'", port);
					return Err(Error::SystemSetupCauseless { msg });
				}
			}
		}

		// Handle Actions
		use warpforge_api::formula::Action;
		let command: Vec<String> = match &formula.action {
			Action::Echo => vec![
				"echo".to_string(),
				"what is the \"Echo\" Action for?".to_string(),
			]
			.to_owned(),
			Action::Execute(a) => a.command.to_owned(),
			Action::Script(a) => self.setup_script(a, &mut mounts)?,
		};

		let random_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
		let ident = format!("warpforge-{random_suffix}");

		let bundle_path = self.executor.ersatz_dir.join(&ident);
		let reference = (formula.image.reference.parse()).map_err(|err| Error::Catchall {
			msg: "failed to parse image reference".into(),
			cause: Box::new(err),
		})?;
		unpack(&reference, &RegistryAuth::Anonymous, &bundle_path)
			.await
			.map_err(|err| Error::SystemSetupError {
				msg: "failed to obtain image".into(),
				cause: Box::new(err),
			})?;

		let params = ContainerParams {
			ident,
			runtime,
			command,
			mounts,
			environment,
			root_path: bundle_path.join("rootfs"),
		};

		self.executor.run(&params, outbox).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;
	use tokio::sync::mpsc;

	#[tokio::test]
	async fn formula_exec_runc_it_works() {
		let formula_and_context: warpforge_api::formula::FormulaAndContext = serde_json::from_str(
			r#"
{
  "formula": {
    "formula.v1": {
      "image": {
        "reference": "docker.io/busybox:latest",
        "readonly": true
      },
      "inputs": {
        "$MSG": "literal:hello from warpforge!"
      },
      "action": {
        "exec": {
          "command": [
            "/bin/sh",
            "-c",
            "echo $MSG"
          ]
        }
      },
      "outputs": {}
    }
  },
  "context": {
    "context.v1": {
      "warehouses": {}
    }
  }
}"#,
		)
		.expect("failed to parse formula json");

		let executor = Executor {
			ersatz_dir: Path::new("/tmp/warpforge-test-executor-runc/run").to_owned(),
			log_file: Path::new("/tmp/warpforge-test-executor-runc/log").to_owned(),
		};
		let (gather_chan, mut gather_chan_recv) = mpsc::channel::<Event>(32);

		let formula = Formula { executor };

		let gather_handle = tokio::spawn(async move {
			while let Some(evt) = gather_chan_recv.recv().await {
				println!("event! {:?}", evt);
				match &evt.body {
					EventBody::Output { channel, val } => {
						assert_eq!(channel, &1);
						assert_eq!(val, "hello from warpforge!");
					}
					EventBody::ExitCode(code) => {
						assert_eq!(code, &Some(0));
					}
				};
			}
		});

		formula
			.run(formula_and_context, "runc".into(), gather_chan)
			.await
			.expect("it didn't fail");
		gather_handle.await.expect("gathering events failed");
	}

	#[tokio::test]
	async fn formula_script_runc_it_works() {
		let formula_and_context: warpforge_api::formula::FormulaAndContext = serde_json::from_str(
			r#"
{
    "formula": {
        "formula.v1": {
            "image": {
              "reference": "docker.io/busybox:latest",
              "readonly": true
            },
            "inputs": {},
            "action": {
                "script": {
                    "interpreter": "/bin/sh",
                    "contents": [
                        "MESSAGE='hello, this is a script action'",
                        "echo $MESSAGE"
                    ]
                }
            },
            "outputs": {
                "test": {
                    "from": "/out",
                    "packtype": "tar"
                }
            }
        }
    },
    "context": {
        "context.v1": {
            "warehouses": {}
        }
    }
}
"#,
		)
		.expect("failed to parse formula json");

		let executor = Executor {
			ersatz_dir: Path::new("/tmp/warpforge-test-formula-executor-runc/run").to_owned(),
			log_file: Path::new("/tmp/warpforge-test-formula-executor-runc/log").to_owned(),
		};
		let (gather_chan, mut gather_chan_recv) = mpsc::channel::<Event>(32);

		let executor = Formula { executor };

		let gather_handle = tokio::spawn(async move {
			let mut output_was_sent = false;
			while let Some(evt) = gather_chan_recv.recv().await {
				println!("event! {:?}", evt);
				match &evt.body {
					EventBody::Output { channel, val } => {
						assert_eq!(channel, &1);
						assert_eq!(val, "hello, this is a script action");
						output_was_sent = true;
					}
					EventBody::ExitCode(code) => {
						assert_eq!(code, &Some(0));
					}
				};
			}
			assert!(output_was_sent);
		});

		executor
			.run(formula_and_context, "runc".into(), gather_chan)
			.await
			.expect("it didn't fail");
		gather_handle.await.expect("gathering events failed");
	}
}
