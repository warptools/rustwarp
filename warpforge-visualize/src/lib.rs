//! This code is a proof of concept for creating a graph of warpforge dependencies.

// There are some useful functions and structs that are not currently used.
// TODO: Remove the following line when cleaning up the code.
#![allow(dead_code)]

use indexmap::IndexMap;
use reqwest::StatusCode;
use reqwest::Url;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt::Display;
use std::fmt::Formatter;

use warpforge_api::plot::Plot;
use warpforge_api::plot::PlotCapsule;
use warpforge_api::plot::PlotInput;
use warpforge_api::plot::Step;

const WARPSYS_URL: &str = "https://raw.githubusercontent.com/warptools/warpsys/master";
const WARPSYS_CATALOG_DOMAIN: &str = "warpsys.org";
const PLOT_FILENAME: &str = "plot.wf";

pub fn graph_dependencies(package: &str) {
	let warpsys_url = Url::parse(WARPSYS_URL).unwrap();
	let plots = fetch_warpsys_plot_recursively(&warpsys_url, package);
	println!("{}", create_graph(&plots, package));
}

#[derive(Default, Clone, Debug)]
pub(crate) struct Graph(Vec<Statement>);

#[derive(Clone, Debug)]
pub(crate) enum Statement {
	Node(Node),
	Edge(Edge),
	SubGraph(String, Graph),
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
	pub(crate) name: String,
	pub(crate) attributes: Vec<NodeAttribute>,
}

#[derive(Clone, Debug)]
pub(crate) struct Edge {
	pub(crate) from: String,
	pub(crate) to: String,
	pub(crate) attributes: Vec<EdgeAttribute>,
}

#[derive(Clone, Debug, derive_more::Display)]
pub(crate) enum NodeAttribute {
	#[display("[label=\"{}\"]", _0)]
	Label(String),

	#[display("[shape={}]", _0)]
	Shape(NodeShape),

	#[display("[color={0},fontcolor={0}]", _0)]
	Color(String),
}

// For more shapes see: https://graphviz.org/doc/info/shapes.html
#[derive(Clone, Debug, derive_more::Display)]
pub(crate) enum NodeShape {
	#[display("box")]
	Box,

	#[display("ellipse")]
	Ellipse,

	#[display("diamond")]
	Diamond,
}

#[derive(Clone, Debug, derive_more::Display)]
pub(crate) enum EdgeAttribute {
	#[display("[label=\"{}\"]", _0)]
	Label(String),

	#[display("[arrowhead={}]", _0)]
	Arrow(ArrowType),

	#[display("[constraint={}]", _0)]
	Constraint(bool),
}

// For more arrow types see: https://graphviz.org/docs/attr-types/arrowType/
#[derive(Clone, Debug, derive_more::Display)]
pub(crate) enum ArrowType {
	#[display("normal")]
	Normal,

	#[display("none")]
	None,

	#[display("empty")]
	Empty,

	#[display("open")]
	Open,
}

impl Graph {
	pub(crate) fn add_node(&mut self, node: Node) {
		self.0.push(Statement::Node(node));
	}

	pub(crate) fn add_edge(&mut self, edge: Edge) {
		self.0.push(Statement::Edge(edge));
	}

	pub(crate) fn add_subgraph(&mut self, name: String, edge: Graph) {
		self.0.push(Statement::SubGraph(name, edge));
	}
}

impl Node {
	pub(crate) fn new(name: String) -> Self {
		Self {
			name,
			attributes: Vec::with_capacity(0),
		}
	}

	pub(crate) fn with_shape(mut self, shape: NodeShape) -> Self {
		self.attributes.push(NodeAttribute::Shape(shape));
		self
	}

	pub(crate) fn with_label(mut self, label: String) -> Self {
		self.attributes.push(NodeAttribute::Label(label));
		self
	}

	pub(crate) fn with_color(mut self, color: String) -> Self {
		self.attributes.push(NodeAttribute::Color(color));
		self
	}
}

impl Edge {
	pub(crate) fn new(from: String, to: String) -> Self {
		Self {
			from,
			to,
			attributes: Vec::with_capacity(0),
		}
	}

	pub(crate) fn with_label(mut self, label: String) -> Self {
		self.attributes.push(EdgeAttribute::Label(label));
		self
	}

	pub(crate) fn with_arrow(mut self, arrow: ArrowType) -> Self {
		self.attributes.push(EdgeAttribute::Arrow(arrow));
		self
	}

	pub(crate) fn with_constraint(mut self, constraint: bool) -> Self {
		self.attributes.push(EdgeAttribute::Constraint(constraint));
		self
	}
}

const IDENT_SPACES: usize = 2;

impl Display for Graph {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "digraph {{")?;
		fmt_entries(&self.0, f, IDENT_SPACES)?;
		writeln!(f, "}}")?;
		Ok(())
	}
}

fn fmt_entries(entries: &[Statement], f: &mut Formatter<'_>, ident: usize) -> std::fmt::Result {
	let prefix = " ".repeat(ident);
	for statement in entries {
		match statement {
			Statement::Node(node) => {
				write!(f, "{prefix}\"{}\"", node.name)?;
				for attribute in &node.attributes {
					write!(f, "{attribute}")?;
				}
				writeln!(f, ";")?;
			}
			Statement::Edge(edge) => {
				write!(f, "{prefix}\"{}\" -> \"{}\"", edge.from, edge.to)?;
				for attribute in &edge.attributes {
					write!(f, "{attribute}")?;
				}
				writeln!(f, ";")?;
			}
			Statement::SubGraph(name, graph) => {
				writeln!(f, "{prefix}subgraph \"cluster_{name}\" {{")?;
				fmt_entries(&graph.0, f, ident + IDENT_SPACES)?;
				writeln!(f, "{prefix}}}")?;
			}
		}
	}
	Ok(())
}

pub(crate) trait Graphable {
	fn graph(&self) -> Graph;
}

impl Graphable for Plot {
	fn graph(&self) -> Graph {
		let mut graph = Graph::default();

		let mut inputs_graph = Graph::default();
		for (label, _) in &self.inputs {
			inputs_graph
				.add_node(Node::new(format!("pipe::{}", label.0)).with_label(label.0.to_string()));
		}
		graph.add_subgraph("inputs".to_string(), inputs_graph);

		for (label, step) in &self.steps {
			let mut subgraph = Graph::default();
			let step_label = label.0.to_string();
			let Step::Protoformula(formula) = step else {
				unimplemented!()
			}; // TODO
			for (target, input) in &formula.inputs {
				if let PlotInput::Pipe(_) = input {
					let input_target_id = format!("{step_label}{}", target.0);
					subgraph
						.add_node(Node::new(input_target_id.clone()).with_label(target.0.clone()));
					subgraph.add_edge(
						Edge::new(input.to_string(), input_target_id.clone())
							.with_arrow(ArrowType::Empty),
					);
					subgraph.add_edge(Edge::new(input_target_id, step_label.clone()));
				}
			}
			subgraph.add_node(Node::new(step_label.clone()).with_shape(NodeShape::Box));
			graph.add_subgraph(step_label, subgraph);
		}

		for i in 1..self.steps.len() {
			graph.add_edge(Edge::new(
				self.steps.get_index(i - 1).unwrap().0 .0.to_string(),
				self.steps.get_index(i).unwrap().0 .0.to_string(),
			));
		}

		graph
	}
}

pub(crate) fn fetch_warpsys_plot(base_url: &Url, package: &str) -> Option<Plot> {
	let mut url = base_url.clone();
	url.path_segments_mut()
		.unwrap()
		.push(package)
		.push(PLOT_FILENAME);
	eprintln!("{url}");
	let plot_file = reqwest::blocking::get(url.clone()).unwrap();
	if plot_file.status() != StatusCode::OK {
		return None;
	}
	let parse_result = serde_json::from_reader(plot_file);
	if parse_result.is_err() {
		eprintln!("Failed to parse '{url}'");
	}
	let PlotCapsule::V1(plot) = parse_result.unwrap();
	Some(plot)
}

pub(crate) fn fetch_warpsys_plot_recursively(
	base_url: &Url,
	package: &str,
) -> IndexMap<String, Plot> {
	let mut failed = HashSet::new();
	let mut result = IndexMap::new();
	let mut next_packages = VecDeque::new();
	next_packages.push_back(package.to_string());
	loop {
		let Some(package) = next_packages.pop_front() else {
			break;
		};
		if result.contains_key(&package) || failed.contains(&package) {
			continue;
		}

		let Some(plot) = fetch_warpsys_plot(base_url, &package) else {
			failed.insert(package);
			continue;
		};

		for (_, input) in &plot.inputs {
			let Some(other_package) = package_name(input) else {
				continue;
			};
			if !result.contains_key(other_package) {
				next_packages.push_back(other_package.to_string());
			}
		}

		result.insert(package, plot);
	}

	eprintln!("failed to load:");
	for fail in failed {
		eprintln!("{fail}");
	}

	result
}

fn package_name(input: &PlotInput) -> Option<&str> {
	let PlotInput::CatalogRef(catalog) = input else {
		return None;
	};
	let mut url = catalog.module_name.0.strip_prefix(WARPSYS_CATALOG_DOMAIN)?;
	while let Some(stripped) = url.strip_prefix('/') {
		url = stripped;
	}
	Some(url)
}

pub(crate) fn create_graph(plots: &IndexMap<String, Plot>, package: &str) -> Graph {
	let mut graph = Graph::default();

	let mut next_packages = VecDeque::new();
	next_packages.push_back(package.to_string());
	let mut visited = HashSet::new();

	loop {
		let Some(package) = next_packages.pop_front() else {
			break;
		};
		if visited.contains(&package) {
			continue;
		}
		visited.insert(package.clone());
		let Some(plot) = plots.get(&package) else {
			continue;
		};

		graph.add_node(Node::new(package.clone()));

		for (_, input) in &plot.inputs {
			let Some(other_package) = package_name(input) else {
				continue;
			};
			if plots.contains_key(other_package) {
				if visited.contains(other_package) {
					let decoupled = format!("{}:{}", package, other_package);
					graph.add_node(
						Node::new(decoupled.clone())
							.with_label(other_package.to_string())
							.with_color("blue".to_string()),
					);
					graph.add_edge(Edge::new(decoupled, package.clone()));
				} else {
					graph.add_edge(Edge::new(other_package.to_string(), package.clone()));
					next_packages.push_back(other_package.to_string());
				}
			} else {
				graph.add_node(Node::new(other_package.to_string()).with_color("red".to_string()));
				graph.add_edge(Edge::new(other_package.to_string(), package.clone()));
			}
		}
	}

	graph
}
