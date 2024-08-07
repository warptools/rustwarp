use indexmap::IndexMap;
use std::io::Write;
use std::path::PathBuf;
use tokio::sync::mpsc;
use warpforge_api::formula::{self, FormulaAndContext};
use warpforge_terminal::logln;

use crate::events::EventBody;
use crate::{runc, Error, Event};

pub enum Formula {
	Runc(runc::Executor),
	//Gvisor(crate::gvisor::GvisorExecutor),
}

pub async fn run_formula(formula: FormulaAndContext) -> Result<(), Error> {
	let temporary_dir = tempfile::tempdir().map_err(|err| Error::SystemSetupError {
		msg: "failed to setup temporary dir".into(),
		cause: Box::new(err),
	})?;

	let executor = Formula::Runc(runc::Executor {
		ersatz_dir: temporary_dir.path().join("run"),
		log_file: temporary_dir.path().join("log"), // TODO: Find a better more persistent location for logs.
	});

	let (event_sender, mut event_receiver) = mpsc::channel::<Event>(32);

	let event_handler = tokio::spawn(async move {
		while let Some(event) = event_receiver.recv().await {
			match &event.body {
				EventBody::Output { val, .. } => logln!("[runc] {val}\n"),
				EventBody::ExitCode(code) => return *code,
			}
		}

		None
	});

	executor.run(formula, event_sender).await?;

	let exit_code = event_handler.await.map_err(|e| Error::SystemRuntimeError {
		msg: "unexpected error while running runc".into(),
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
		a: &warpforge_api::formula::ActionScript,
		mounts: &mut indexmap::IndexMap<std::ffi::OsString, crate::MountSpec>,
	) -> Result<Vec<std::string::String>, crate::Error> {
		let ersatz_dir = match self {
			self::Formula::Runc(crate::runc::Executor { ersatz_dir, .. }) => ersatz_dir,
		};

		let script_dir = ersatz_dir.join("script");
		use std::fs;
		fs::create_dir_all(&script_dir).map_err(|e| {
			let msg = "failed during formula execution: couldn't create script dir".to_owned();
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

		let mut script_file =
			fs::File::create(script_dir.join("run")).map_err(|e| crate::Error::Catchall {
				msg: "failed during formula execution: couldn't open script file for writing"
					.to_owned(),
				cause: Box::new(e),
			})?;

		for (n, line) in a.contents.iter().enumerate() {
			let entry_file_name = format!("entry-{}", n);
			let mut entry_file =
				fs::File::create(script_dir.join(&entry_file_name)).map_err(|e| {
					crate::Error::Catchall {
						msg: "failed during formula execution: couldn't cr".to_owned()
							+ &format!("eate script entry number {}", n).to_string(),
						cause: Box::new(e),
					}
				})?;
			writeln!(entry_file, "{}", line).map_err(|e| crate::Error::Catchall {
				msg: format!(
					"failed during formula execution: io error writing script entry file {n}",
				),
				cause: Box::new(Into::<std::io::Error>::into(e)),
			})?;
			writeln!(
				script_file,
				". {}",
				Self::container_script_path()
					.join(entry_file_name)
					.display()
			)
			.map_err(|e| crate::Error::Catchall {
				msg: format!(
					"failed during formula execution: io error writing script file entry {n}"
				),
				cause: Box::new(Into::<std::io::Error>::into(e)),
			})?;
		}

		// mount the script into the container
		mounts.insert(
			Self::container_script_path().as_os_str().to_owned(),
			crate::MountSpec::new_bind(&script_dir, &Self::container_script_path(), false),
		);

		Ok(vec![
			a.interpreter.to_owned(),
			Self::container_script_path()
				.join("run")
				.display()
				.to_string(),
		])
	}

	pub async fn run(
		&self,
		formula_and_context: warpforge_api::formula::FormulaAndContext,
		outbox: tokio::sync::mpsc::Sender<crate::Event>,
	) -> Result<(), crate::Error> {
		let mut mounts = IndexMap::new();
		let mut environment = IndexMap::new();
		let formula::FormulaCapsule::V1(formula) = formula_and_context.formula;

		// Handle Inputs
		for (formula::SandboxPort(port), input) in formula.inputs {
			//TODO implement the FormulaInputComplex filter thing
			match port.get(..1) {
				// TODO replace this with a catverter macro
				Some("$") => {
					environment.insert(
						port.get(1..)
							.expect("environment variable with empty name")
							.into(),
						match input {
							warpforge_api::plot::PlotInput::Literal(l) => l,
							_ => panic!(
								"input environment variable value {}",
								"contains invalid discriminant"
							),
						}
						.into(),
					);
				}
				Some("/") => {}
				None | Some(_) => {}
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

		let params = crate::ContainerParams {
			command,
			mounts,
			environment,
			root_path: "/tmp/rootfs".to_string(),
		};
		match self {
			Formula::Runc(e) => e.run(&params, outbox).await,
			/*Formula::Gvisor(_e) => Err(crate::Error::CatchallCauseless {
				msg: "gvisor executor not implemented".to_string(),
			}),*/
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	use serial_test::serial;
	use tokio::sync::mpsc;

	#[tokio::test]
	#[serial(rootfs)]
	async fn formula_exec_runc_it_works() {
		let formula_and_context: warpforge_api::formula::FormulaAndContext =
			serde_json::from_str(
				r#"
{
  "formula": {
    "formula.v1": {
      "inputs": {
        "/": "ware:tar:4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9",
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
      "warehouses": {
        "tar:4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9": "https://warpsys.s3.amazonaws.com/warehouse/4z9/DCT/4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9"
      }
    }
  }
}"#,
			).expect("failed to parse formula json");

		let cfg = crate::runc::Executor {
			ersatz_dir: Path::new("/tmp/warpforge-test-executor-runc/run").to_owned(),
			log_file: Path::new("/tmp/warpforge-test-executor-runc/log").to_owned(),
		};
		let (gather_chan, mut gather_chan_recv) = mpsc::channel::<crate::events::Event>(32);

		let executor = Formula::Runc(cfg);

		// empty gather_chan
		let gather_handle = tokio::spawn(async move {
			while let Some(evt) = gather_chan_recv.recv().await {
				println!("event! {:?}", evt);
				match &evt.body {
					crate::events::EventBody::Output { channel, val } => {
						assert_eq!(channel, &1);
						assert_eq!(val, "hello from warpforge!");
					}
					crate::events::EventBody::ExitCode(code) => {
						assert_eq!(code, &Some(0));
						// return; // stop processing events (this breaks it?)
					}
				};
			}
		});

		executor
			.run(formula_and_context, gather_chan)
			.await
			.expect("it didn't fail");
		gather_handle.await.expect("gathering events failed");
	}

	#[tokio::test]
	#[serial(rootfs)]
	async fn formula_script_runc_it_works() {
		let formula_and_context: warpforge_api::formula::FormulaAndContext =
			serde_json::from_str(
				r#"
{
	"formula": {
		"formula.v1": {
			"inputs": {
				"/": "ware:tar:4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9"
			},
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
			"warehouses": {
				"tar:4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9": "https://warpsys.s3.amazonaws.com/warehouse/4z9/DCT/4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9"
			}
		}
	}
}
"#,
			).expect("failed to parse formula json");

		let cfg = crate::runc::Executor {
			ersatz_dir: Path::new("/tmp/warpforge-test-formula-executor-runc/run").to_owned(),
			log_file: Path::new("/tmp/warpforge-test-formula-executor-runc/log").to_owned(),
		};
		let (gather_chan, mut gather_chan_recv) = mpsc::channel::<crate::events::Event>(32);

		let executor = Formula::Runc(cfg);

		// empty gather_chan
		let gather_handle = tokio::spawn(async move {
			let mut output_was_sent = false; // gross but i can't think of a better way
			while let Some(evt) = gather_chan_recv.recv().await {
				println!("event! {:?}", evt);
				match &evt.body {
					crate::events::EventBody::Output { channel, val } => {
						assert_eq!(channel, &1);
						assert_eq!(val, "hello, this is a script action");
						output_was_sent = true;
					}
					crate::events::EventBody::ExitCode(code) => {
						assert_eq!(code, &Some(0));
						//return; // stop processing events
					}
				};
			}
			assert!(output_was_sent);
		});

		executor
			.run(formula_and_context, gather_chan)
			.await
			.expect("it didn't fail");
		gather_handle.await.expect("gathering events failed");
	}
}
