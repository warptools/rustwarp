use indexmap::{IndexMap, IndexSet};
use warpforge_api::plot::{PlotCapsule, PlotInput, Step, StepName};
use warpforge_terminal::logln;

use crate::context::Context;
use crate::{Error, Result};

pub async fn run_plot(plot: PlotCapsule, _context: &Context) -> Result<()> {
	let graph = PlotGraph::new(&plot);
	graph.validate()?;
	logln!("{graph:#?}");

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
