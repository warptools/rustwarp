use indexmap::{IndexMap, IndexSet};
use tempfile::TempDir;
use warpforge_api::content::Packtype;
use warpforge_api::formula::{
	Formula, FormulaAndContext, FormulaCapsule, FormulaContext, FormulaContextCapsule,
	FormulaInput, GatherDirective, Mount,
};
use warpforge_api::plot::{PlotCapsule, PlotInput, Step, StepName};
use warpforge_terminal::logln;

use crate::context::Context;
use crate::formula::run_formula;
use crate::{to_string_or_panic, Error, Output, Result};

pub async fn run_plot(plot: PlotCapsule, context: &Context) -> Result<()> {
	let graph = PlotGraph::new(&plot);
	graph.validate()?;

	let temp_dir = TempDir::new().map_err(|err| Error::SystemSetupError {
		msg: "failed to setup temporary dir".into(),
		cause: Box::new(err),
	})?;

	// TODO: Execute in graph order.

	let PlotCapsule::V1(plot) = &plot;
	for (StepName(step_name), step) in &plot.steps {
		let Step::Protoformula(step) = step else {
			todo!(); // TODO: Implement sub-plots.
		};

		let step_dir = temp_dir.path().join(step_name);
		let output_path = Some(step_dir.join("outputs"));
		let context = Context {
			output_path,
			..context.clone()
		};

		let image = plot.image.as_ref().or(step.image.as_ref());
		let Some(image) = image else {
			let msg = format!("invalid plot (step '{step_name}'): image required");
			return Err(Error::SystemSetupCauseless { msg });
		};

		let inputs = (step.inputs.iter())
			.map(|(port, input)| {
				let input = match input {
					PlotInput::Mount(mount) => FormulaInput::Mount(mount.to_owned()),
					PlotInput::Literal(literal) => FormulaInput::Literal(literal.to_owned()),
					PlotInput::Ware(ware_id) => FormulaInput::Ware(ware_id.to_owned()),
					PlotInput::Pipe(pipe) => {
						if pipe.step_name.is_empty() {
							todo!();
						}
						let path = (temp_dir.path())
							.join(&pipe.step_name)
							.join("outputs")
							.join(&pipe.label.0);
						FormulaInput::Mount(Mount::ReadOnly(to_string_or_panic(path)))
					}
					PlotInput::CatalogRef(_catalog_ref) => todo!(),
					PlotInput::Ingest(_ingest) => todo!(),
				};
				(port.to_owned(), input)
			})
			.collect::<IndexMap<_, _>>();

		for (_, GatherDirective { packtype, .. }) in &step.outputs {
			if (packtype.as_ref())
				.map(|Packtype(p)| p != "none")
				.unwrap_or(false)
			{
				let msg =
					format!("invalid plot (step '{step_name}'): output packtype has to be 'none'");
				return Err(Error::SystemSetupCauseless { msg });
			}
		}

		let formula = Formula {
			image: image.clone(),
			inputs,
			action: step.action.clone(),
			outputs: step.outputs.clone(),
		};

		let formula = FormulaAndContext {
			formula: FormulaCapsule::V1(formula),
			context: FormulaContextCapsule::V1(FormulaContext {
				warehouses: IndexMap::with_capacity(0),
			}),
		};

		let outputs = run_formula(formula, &context).await.map_err(|err| {
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
	}

	Ok(())
}

#[derive(Debug)]
pub(crate) struct PlotGraph<'a> {
	nodes: IndexMap<&'a str, &'a Step>,
	parents: IndexMap<&'a str, IndexSet<&'a str>>,
	children: IndexMap<&'a str, IndexSet<&'a str>>,
}

impl<'a> PlotGraph<'a> {
	pub(crate) fn new(plot: &'a PlotCapsule) -> Self {
		let mut parents = IndexMap::new();
		let mut children = IndexMap::new();
		let mut nodes = IndexMap::new();

		let PlotCapsule::V1(plot) = plot;
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
				let removed = child_parents.remove(node);
				if removed && child_parents.is_empty() {
					parents.remove(child);
					no_parents.push(child);
				}
			}
		}
		Ok(())
	}
}
