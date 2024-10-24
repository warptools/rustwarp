use indexmap::{IndexMap, IndexSet};
use tempfile::TempDir;
use warpforge_api::formula::{
	Formula, FormulaAndContext, FormulaCapsule, FormulaContext, FormulaContextCapsule,
	FormulaInput, GatherDirective, Mount,
};
use warpforge_api::plot::{LocalLabel, Plot, PlotCapsule, PlotInput, PlotOutput, Step, StepName};
use warpforge_terminal::{logln, Bar};

use crate::context::Context;
use crate::formula::run_formula;
use crate::pack::{pack_outputs, IntermediateOutput, OutputPacktype};
use crate::{to_string_or_panic, Error, Output, Result};

const OUTPUTS_DIR: &str = "outputs";

pub fn run_plot(plot: PlotCapsule, context: &Context) -> Result<Vec<Output>> {
	let PlotCapsule::V1(plot) = &plot;

	let graph = PlotGraph::new(plot);
	graph.validate()?;

	let temp_dir = TempDir::new().map_err(|err| Error::SystemSetupError {
		msg: "failed to setup temporary dir".into(),
		cause: Box::new(err),
	})?;

	PlotExecutor {
		context,
		plot,
		graph,
		temp_dir,
	}
	.run()
}

#[allow(unused)]
struct PlotExecutor<'a> {
	context: &'a Context,
	plot: &'a Plot,
	graph: PlotGraph<'a>,
	temp_dir: TempDir,
}

impl<'a> PlotExecutor<'a> {
	fn run(&self) -> Result<Vec<Output>> {
		let progress = Bar::new(self.plot.steps.len() as u64, "");

		let mut parents = self.graph.parents.clone();
		let mut next_steps = (self.graph.nodes.keys().cloned())
			.filter(|name| match parents.get(name) {
				Some(node_parents) => node_parents.is_empty(),
				None => true,
			})
			.collect::<Vec<_>>();

		// TODO: Run multiple steps in parallel, when possible.
		let mut completed_count = 0;
		while let Some(step_name) = next_steps.pop() {
			progress.set_text(step_name);

			self.run_step(step_name)?;

			completed_count += 1;
			progress.set_position(completed_count);

			let Some(children) = self.graph.children.get(step_name) else {
				continue;
			};
			for &child in children {
				let child_parents = &mut parents[child];
				let removed = child_parents.swap_remove(step_name);
				if removed && child_parents.is_empty() {
					next_steps.push(child);
				}
			}
		}

		let mut outputs = Vec::new();
		for (LocalLabel(name), PlotOutput::Pipe(pipe)) in &self.plot.outputs {
			let Some(Step::Protoformula(step)) = self.plot.steps.get(&pipe.step_name) else {
				let msg = format!("output '{name}': step '{}' not found", pipe.step_name);
				return Err(Error::SystemSetupCauseless { msg });
			};
			let Some(step_output) = step.outputs.get(&pipe.label) else {
				let msg = format!(
					"output '{name}': step '{}' has no output named '{}'",
					pipe.step_name, pipe.label,
				);
				return Err(Error::SystemSetupCauseless { msg });
			};

			let host_path = (self.temp_dir.path())
				.join(&pipe.step_name)
				.join(OUTPUTS_DIR)
				.join(&pipe.label.0);
			outputs.push(IntermediateOutput {
				name: name.to_owned(),
				host_path,
				packtype: OutputPacktype::parse(&step_output.packtype)?,
			});
		}

		pack_outputs(&self.context.output_path, &outputs)
	}

	fn run_step(&self, step_name: &str) -> Result<()> {
		let Step::Protoformula(step) = self.graph.nodes[step_name] else {
			todo!(); // TODO: Implement sub-plots.
		};

		let step_dir = self.temp_dir.path().join(step_name);
		let context = Context {
			output_path: Some(step_dir.join(OUTPUTS_DIR)),
			..self.context.clone()
		};

		let image = self.plot.image.as_ref().or(step.image.as_ref());
		let Some(image) = image else {
			let msg = format!("invalid plot (step '{step_name}'): image required");
			return Err(Error::SystemSetupCauseless { msg });
		};

		let mut inputs = IndexMap::new();
		for (port, input) in &step.inputs {
			let input = match input {
				PlotInput::Mount(mount) => FormulaInput::Mount(mount.to_owned()),
				PlotInput::Literal(literal) => FormulaInput::Literal(literal.to_owned()),
				PlotInput::Ware(ware_id) => FormulaInput::Ware(ware_id.to_owned()),
				PlotInput::Pipe(pipe) => {
					if pipe.step_name.is_empty() {
						let Some(plot_input) = self.plot.inputs.get(&pipe.label) else {
							let msg = format!(
								"invalid plot (step '{step_name}'): input '{}' not found",
								pipe.label
							);
							return Err(Error::SystemSetupCauseless { msg });
						};
						match plot_input {
							PlotInput::Mount(mount) => FormulaInput::Mount(mount.to_owned()),
							PlotInput::Ware(ware_id) => FormulaInput::Ware(ware_id.to_owned()),
							PlotInput::Literal(literal) => {
								FormulaInput::Literal(literal.to_owned())
							}
							PlotInput::Pipe(_) => {
								let msg = "invalid plot: plot inputs may not contain pipes".into();
								return Err(Error::SystemSetupCauseless { msg });
							}
							_ => todo!(),
						}
					} else {
						let path = (self.temp_dir.path())
							.join(&pipe.step_name)
							.join(OUTPUTS_DIR)
							.join(&pipe.label.0);
						FormulaInput::Mount(Mount::ReadOnly(to_string_or_panic(path)))
					}
				}
				PlotInput::CatalogRef(_catalog_ref) => todo!(),
				PlotInput::Ingest(_ingest) => todo!(),
			};

			inputs.insert(port.to_owned(), input);
		}

		let outputs = (step.outputs.iter())
			.map(|(label, output)| {
				let output = GatherDirective {
					from: output.from.to_owned(),
					packtype: None,
				};
				(label.to_owned(), output)
			})
			.collect::<IndexMap<_, _>>();

		let formula = Formula {
			image: image.clone(),
			inputs,
			action: step.action.clone(),
			outputs,
		};
		let formula = FormulaAndContext {
			formula: FormulaCapsule::V1(formula),
			context: FormulaContextCapsule::V1(FormulaContext {
				warehouses: IndexMap::with_capacity(0),
			}),
		};
		let outputs = run_formula(formula, &context).map_err(|err| {
			let msg = format!("failed step '{step_name}'");
			let cause = Box::new(err);
			Error::SystemRuntimeError { msg, cause }
		})?;

		logln!("step '{step_name}'");
		for output in outputs {
			let Output {
				name,
				digest: crate::Digest::Sha384(digest),
			} = output;
			logln!("  sha384:{digest} {name}");
		}

		Ok(())
	}
}

#[derive(Debug)]
pub(crate) struct PlotGraph<'a> {
	nodes: IndexMap<&'a str, &'a Step>,
	parents: IndexMap<&'a str, IndexSet<&'a str>>,
	children: IndexMap<&'a str, IndexSet<&'a str>>,
}

impl<'a> PlotGraph<'a> {
	pub(crate) fn new(plot: &'a Plot) -> Self {
		let mut parents = IndexMap::new();
		let mut children = IndexMap::new();
		let mut nodes = IndexMap::new();

		for (StepName(name), step) in &plot.steps {
			nodes.insert(name.as_str(), step);
			match step {
				Step::Plot(_sub_plot) => todo!(),
				Step::Protoformula(protoformula) => {
					for (_, input) in &protoformula.inputs {
						let PlotInput::Pipe(pipe) = input else {
							continue;
						};

						if pipe.step_name.is_empty() {
							continue;
						}

						parents
							.entry(name.as_str())
							.or_insert_with(IndexSet::new)
							.insert(pipe.step_name.as_str());
						children
							.entry(pipe.step_name.as_str())
							.or_insert_with(IndexSet::new)
							.insert(name.as_str());
					}
				}
			}
		}

		Self {
			nodes,
			parents,
			children,
		}
	}

	pub(crate) fn validate(&self) -> Result<()> {
		self.validate_dependencies_exist()?;
		self.validate_no_cycles()?;
		Ok(())
	}

	pub(crate) fn validate_dependencies_exist(&self) -> Result<()> {
		for &name in self.children.keys() {
			if !self.nodes.contains_key(name) {
				let origin = self.children[name]
					.iter()
					.cloned()
					.collect::<Vec<_>>()
					.join("', '");
				let msg =
					format!("invalid plot: step(s) '{origin}' reference(s) unknown step '{name}'");
				return Err(Error::SystemSetupCauseless { msg });
			}
		}
		Ok(())
	}

	/// Topological sort to find cycles.
	pub(crate) fn validate_no_cycles(&self) -> Result<()> {
		let mut order = Vec::with_capacity(self.nodes.len());
		let mut parents = self.parents.clone();
		let mut no_parents = (self.nodes.keys().cloned())
			.filter(|name| match parents.get(name) {
				Some(node_parents) => node_parents.is_empty(),
				None => true,
			})
			.collect::<Vec<_>>();

		while order.len() < self.nodes.len() {
			let Some(node) = no_parents.pop() else {
				let cycles = (parents.iter())
					.filter(|(_, child_parents)| !child_parents.is_empty())
					.map(|(&child_name, _)| child_name)
					.collect::<Vec<_>>()
					.join("', '");
				let msg = format!("invalid plot: the step(s) '{cycles}' contain(s) cycle(s)");
				return Err(Error::SystemSetupCauseless { msg });
			};

			// Adding a node each iteration: no endless loop
			order.push(node);

			let Some(children) = self.children.get(node) else {
				continue;
			};
			for &child in children {
				let child_parents = &mut parents[child];
				let removed = child_parents.swap_remove(node);
				if removed && child_parents.is_empty() {
					parents.swap_remove(child);
					no_parents.push(child);
				}
			}
		}
		Ok(())
	}
}
