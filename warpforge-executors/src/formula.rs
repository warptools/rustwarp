use crossbeam_channel::Sender;
use indexmap::IndexMap;
use oci_unpack::{pull_and_unpack, PullConfig};
use rand::distributions::{Alphanumeric, DistString};
use std::io::Write;
use std::path::PathBuf;
use std::{fs, thread};
use warpforge_api::content::Packtype;
use warpforge_api::formula::{
	self, Action, ActionScript, FormulaAndContext, FormulaInput, GatherDirective, Mount,
	SandboxPort,
};
use warpforge_api::plot::LocalLabel;
use warpforge_terminal::{logln, Bar};

use crate::context::Context;
use crate::events::EventBody;
use crate::execute::Executor;
use crate::pack::{pack_outputs, IntermediateOutput, OutputPacktype};
use crate::{to_string_or_panic, ContainerParams, Error, Event, MountSpec, Output, Result};

pub struct Formula<'a> {
	pub(crate) executor: Executor,
	pub(crate) context: &'a Context,
}

pub fn run_formula(formula: FormulaAndContext, context: &Context) -> Result<Vec<Output>> {
	let temporary_dir = tempfile::tempdir().map_err(|err| Error::SystemSetupError {
		msg: "failed to setup temporary dir".into(),
		cause: Box::new(err),
	})?;

	let executor = Formula {
		executor: Executor {
			ersatz_dir: temporary_dir.path().join("run"),
			log_file: temporary_dir.path().join("log"), // TODO: Find a better more persistent location for logs.
		},
		context,
	};

	let (event_sender, event_receiver) = crossbeam_channel::bounded::<Event>(32);

	let event_handler = thread::spawn(move || {
		while let Ok(event) = event_receiver.recv() {
			match &event.body {
				EventBody::Output { val, .. } => logln!("[container] {val}\n"),
				EventBody::ExitCode(code) => return *code,
			}
		}

		None
	});

	let outputs = executor.run(formula, event_sender)?;

	let exit_code = event_handler.join().unwrap();
	match exit_code {
		Some(0) => Ok(outputs),
		_ => Err(Error::SystemRuntimeError {
			msg: "container terminated non-zero exit code".into(),
			cause: exit_code.map_or_else(|| "None".into(), |code| format!("{code}").into()),
		}),
	}
}

impl<'a> Formula<'a> {
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
		let script_path = Self::container_script_path();
		mounts.insert(
			to_string_or_panic(&script_path),
			MountSpec::new_bind(self.context, &script_dir, &script_path, false)?,
		);

		Ok(vec![
			script.interpreter.to_owned(),
			to_string_or_panic(script_path.join("run")),
		])
	}

	pub fn run(
		&self,
		formula_and_context: FormulaAndContext,
		outbox: Sender<Event>,
	) -> Result<Vec<Output>> {
		let formula::FormulaCapsule::V1(formula) = formula_and_context.formula;

		let progress = Bar::new(5, "setup container");

		let (mut mounts, environment) = self.setup_inputs(formula.inputs)?;

		let outputs = self.setup_outputs(formula.outputs, &mut mounts)?;

		// Handle Actions
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

		progress.set(1, "fetch container");

		let bundle_path = self.executor.ersatz_dir.join(&ident);
		let reference = (formula.image.reference.parse()).map_err(|err| Error::Catchall {
			msg: "failed to parse image reference".into(),
			cause: Box::new(err),
		})?;
		let pull_config = PullConfig {
			cache: self.context.image_cache.clone(),
			..PullConfig::default()
		};
		pull_and_unpack(&reference, &bundle_path, &pull_config).map_err(|err| {
			Error::SystemSetupError {
				msg: "failed to obtain image".into(),
				cause: Box::new(err),
			}
		})?;

		progress.set(3, "run container");

		let params = ContainerParams {
			ident,
			runtime: self.context.runtime.clone(),
			command,
			mounts,
			environment,
			root_path: bundle_path.join("rootfs"),
		};
		self.executor.run(&params, outbox)?;

		progress.set(5, "pack outputs");

		pack_outputs(&self.context.output_path, &outputs)
	}

	/// Create all input mounts and collect environment variable inputs.
	fn setup_inputs(
		&self,
		formula_inputs: IndexMap<SandboxPort, FormulaInput>,
	) -> Result<(IndexMap<String, MountSpec>, IndexMap<String, String>)> {
		let mut mounts = IndexMap::new();
		let mut environment = IndexMap::new();

		for (formula::SandboxPort(port), input) in formula_inputs {
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
				Some("/") => {
					match input {
						FormulaInput::Ware(_ware_id) => todo!(),
						FormulaInput::Mount(Mount::ReadOnly(host_path)) => {
							let mount_spec =
								MountSpec::new_bind(self.context, host_path, &port, true)?;
							mounts.insert(port, mount_spec);
						}
						FormulaInput::Mount(Mount::ReadWrite(host_path)) => {
							let mount_spec =
								MountSpec::new_bind(self.context, host_path, &port, false)?;
							mounts.insert(port, mount_spec);
						}
						FormulaInput::Mount(Mount::Overlay(host_path)) => {
							let run_dir = &self.executor.ersatz_dir;
							let mount_spec =
								MountSpec::new_overlayfs(self.context, host_path, &port, run_dir)?;
							mounts.insert(port, mount_spec);
						}
						FormulaInput::Literal(_) => {
							let msg = format!("formula input '{}': 'literal' not supported, use 'ware' or 'mount'", port);
							return Err(Error::SystemSetupCauseless { msg });
						}
					}
				}
				_ => {
					let msg = format!("invalid formula input '{}'", port);
					return Err(Error::SystemSetupCauseless { msg });
				}
			}
		}

		Ok((mounts, environment))
	}

	/// Create writable mounts for all outputs.
	fn setup_outputs(
		&self,
		formula_outputs: IndexMap<LocalLabel, GatherDirective>,
		mounts: &mut IndexMap<String, MountSpec>,
	) -> Result<Vec<IntermediateOutput>> {
		let mut outputs = Vec::new();
		let outputs_dir = self.executor.ersatz_dir.join("outputs");
		for output in formula_outputs {
			let (
				LocalLabel(name),
				GatherDirective {
					from: SandboxPort(port),
					packtype,
				},
			) = output;

			if !port.starts_with('/') {
				let msg = format!("formula output '{name}': 'from' has to contain absolute path");
				return Err(Error::SystemSetupCauseless { msg });
			}

			if mounts.contains_key(&port) {
				let msg = format!("formula output '{name}': duplicate mount path '{port}'");
				return Err(Error::SystemSetupCauseless { msg });
			}

			let packtype = match packtype {
				None => OutputPacktype::None,
				Some(Packtype(p)) if p == "none" => OutputPacktype::None,
				Some(Packtype(p)) if p == "tar" => OutputPacktype::Tar,
				_ => {
					let msg = format!(
						"formula output '{name}': unsupported packtype (allowed values: 'none', 'tar')"
					);
					return Err(Error::SystemSetupCauseless { msg });
				}
			};

			let output_dir = outputs_dir.join(&name);
			fs::create_dir_all(&output_dir).map_err(|err| Error::SystemSetupError {
				msg: format!("formula output '{name}': failed to create output directory"),
				cause: Box::new(err),
			})?;

			let mount_spec = MountSpec::new_bind(self.context, &output_dir, &port, false)?;
			mounts.insert(port, mount_spec);
			outputs.push(IntermediateOutput {
				name,
				host_path: output_dir,
				packtype,
			});
		}

		Ok(outputs)
	}
}
