use std::mem;

use indexmap::{IndexMap, IndexSet};
use json_with_position::TargetHint;
use oci_client::Reference;

use crate::{
	common::{expect_key, expect_object_iterate, expect_string},
	error::{ValidationErrorWithPath, VecValidationErrorWithPath},
	formula::FormulaValidator,
};

pub(crate) struct PlotValidator<'a> {
	graph_builder: PlotGraphBuilder<'a>,
	formula_validators: IndexMap<&'a str, FormulaValidator>,

	/// Order in which plot steps should be run, determined by topological sort.
	step_order: Vec<&'a str>,
}

#[derive(Default)]
struct PlotGraphBuilder<'a> {
	graph: PlotGraph<'a>,
}

impl<'a> PlotGraphBuilder<'a> {
	fn new() -> Self {
		Default::default()
	}

	fn add_step(&mut self, step_name: &'a str, step: PlotStep<'a>) {
		self.graph.steps.insert(step_name, step);
	}

	fn finish(self) -> PlotGraph<'a> {
		self.graph
	}
}

/// Graph used to validate that the steps do NOT have a cyclic dependency and
/// that local dependencies exist.
#[derive(Default)]
struct PlotGraph<'a> {
	steps: IndexMap<&'a str, PlotStep<'a>>,
}

#[derive(Default)]
struct PlotStep<'a> {
	input_pipes: Vec<InputPipe<'a>>,
	outputs: Vec<&'a str>,
}

/// Represents a pipe, which is the input to a protoformula.
///
/// Format in json: "port": "pipe:step:output"
struct InputPipe<'a> {
	port: &'a str,
	step: &'a str,
	name: &'a str,
}

impl<'a> PlotValidator<'a> {
	pub(crate) fn validate(parsed: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		Self {
			graph_builder: PlotGraphBuilder::new(),
			formula_validators: IndexMap::new(),
			step_order: Vec::new(),
		}
		.check(parsed)
	}

	fn check(mut self, value: &'a serde_json::Value) -> Vec<ValidationErrorWithPath> {
		let mut inputs = None;
		let mut outputs = None;

		let mut errors = expect_key(value, "plot.v1", |value| {
			let mut errors = expect_key(value, "inputs", |value| {
				inputs = Some(value);
				self.check_inputs(value)
			});
			errors.extend(expect_key(value, "steps", |value| self.check_steps(value)));
			errors.extend(expect_key(value, "outputs", |value| {
				outputs = Some(value);
				Vec::with_capacity(0)
			}));

			errors
		});

		if let (Some(inputs), Some(outputs)) = (inputs, outputs) {
			errors.extend(self.check_graph_and_outputs(inputs, outputs));
		}

		errors
	}

	fn check_inputs(&self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		expect_object_iterate(value, |(_key, value)| {
			expect_string(value, |value| {
				let mut parts = value.split(':');
				let discriminant = parts.next().expect("split emits at least one value");

				match discriminant {
					"literal" => {
						if parts.next().is_none() {
							return ValidationErrorWithPath::build(
								"input type 'literal' requires value",
							)
							.with_label("invalid literal")
							.with_note("example input: \"msg\": \"literal:Hello, World!\"")
							.finish();
						}
					}
					"mount" => {
						let (Some(mount_type), Some(_host_path)) = (parts.next(), parts.next())
						else {
							return ValidationErrorWithPath::build(
								"input type 'mount' requires mount type and host path",
							)
							.with_label("invalid mount")
							.with_note("example mount: \"name\": \"mount:ro:/host/path\"")
							.finish();
						};

						if !["ro", "rw", "overlay"].contains(&mount_type) {
							return ValidationErrorWithPath::build(
								"mount type not allowed (allowed types: 'ro', 'rw', 'overlay')",
							)
							.with_label("mount with invalid mount type")
							.with_note("example mount: \"name\": \"mount:ro:/host/path\"")
							.finish();
						}
					}
					"oci" => {
						let Some(oci) = parts.next() else {
							return ValidationErrorWithPath::build(
								"input type 'oci' requires oci reference",
							)
							.with_label("invalid oci input")
							.with_note("example input: \"name\": \"oci:docker.io/busybox:latest\"")
							.finish();
						};

						let reference = match oci.parse::<Reference>() {
							Ok(reference) => reference,
							Err(err) => {
								return ValidationErrorWithPath::custom(format!(
									"failed to parse oci reference: {err}"
								));
							}
						};

						// TODO: resolve oci digest
					}
					_ => {
						let message =
							"input type not allowed (allowed types: 'literal', 'mount', 'oci')";
						return ValidationErrorWithPath::build(message)
							.with_label("invalid plot input")
							.finish();
					}
				}

				Vec::with_capacity(0)
			})
		})
	}

	fn check_steps(&mut self, value: &'a serde_json::Value) -> Vec<ValidationErrorWithPath> {
		expect_object_iterate(value, |(step_name, value)| {
			let mut errors = expect_key(value, "protoformula", |value| {
				let mut step: PlotStep<'a> = Default::default();

				// Ignoring errors here, because we check this
				// structure in the FormulaValidator already.
				let _ = expect_key(value, "inputs", |value| {
					expect_object_iterate(value, |(port, value)| {
						expect_string(value, |value| {
							if let Some(pipe) = value.strip_prefix("pipe:") {
								let mut parts = pipe.split(':');
								if let (Some(input_step), Some(output), None) =
									(parts.next(), parts.next(), parts.next())
								{
									step.input_pipes.push(InputPipe {
										port,
										step: input_step,
										name: output,
									});
								} else {
									return ValidationErrorWithPath::build("expected step and output")
										.with_label("invalid pipe")
										.with_note("example pipe: \"name\": \"pipe:step_name:step_output_name\"")
										.finish();
								}
							}
							Vec::with_capacity(0)
						})
					})
				});
				let _ = expect_key(value, "outputs", |value| {
					expect_object_iterate(value, |(output_name, _value)| {
						step.outputs.push(output_name);
						Vec::with_capacity(0)
					})
				});

				self.graph_builder.add_step(step_name, step);

				let mut validator = FormulaValidator::new(true);
				let errors = validator.check(value);
				self.formula_validators.insert(step_name, validator);
				errors
			});

			if step_name.is_empty() {
				let key_error = ValidationErrorWithPath::build("empy step name not allowed")
					.with_label("specify step name here")
					.with_target(TargetHint::Key)
					.finish();
				errors.extend(key_error);
			}

			errors
		})
	}

	fn check_graph_and_outputs(
		&mut self,
		plot_inputs: &serde_json::Value,
		plot_outputs: &serde_json::Value,
	) -> Vec<ValidationErrorWithPath> {
		let graph = mem::take(&mut self.graph_builder).finish();

		let mut errors = self.check_input_pipes_valid(&graph, plot_inputs);
		errors.extend(self.check_graph_not_cyclic(&graph));

		let mut outputs_errors = self.check_outputs(plot_outputs, &graph);
		outputs_errors.prepend_object_index("outputs");
		errors.extend(outputs_errors);

		errors.prepend_object_index("plot.v1");
		errors
	}

	fn check_input_pipes_valid(
		&self,
		graph: &PlotGraph<'a>,
		plot_inputs: &serde_json::Value,
	) -> Vec<ValidationErrorWithPath> {
		let mut step_errors = Vec::with_capacity(0);

		for (target_step_name, step) in &graph.steps {
			let mut input_errors = Vec::with_capacity(0);

			for pipe in &step.input_pipes {
				if !pipe.step.is_empty() {
					if pipe.port.starts_with('$') {
						let mut error = ValidationErrorWithPath::build(
							"env variable may only be piped from plot inputs",
						)
						.with_label("invalid pipe")
						.with_note("change port into a mount type or remove step name from pipe")
						.with_target(TargetHint::Key)
						.finish();
						error.prepend_object_index(pipe.port);
						input_errors.extend(error);
					}

					let Some(step) = graph.steps.get(pipe.step) else {
						let mut error =
							ValidationErrorWithPath::build("pipe has invalid step name")
								.with_label("invalid pipe")
								.finish();
						error.prepend_object_index(pipe.port);
						input_errors.extend(error);
						continue;
					};

					if !step.outputs.contains(&pipe.name) {
						let mut error = ValidationErrorWithPath::build(
							"step does not contain specified output",
						)
						.with_label("invalid pipe")
						.finish();
						error.prepend_object_index(pipe.port);
						input_errors.extend(error);
					}
				} else {
					// Check if pipes with format "pipe::plot_input"
					// reference a plot input of a correct input type.
					let Some(plot_input) = plot_inputs.as_object().and_then(|o| o.get(pipe.name))
					else {
						let mut error = ValidationErrorWithPath::build("plot input for pipe not found")
							.with_label("invalid pipe")
							.with_note("make sure a plot input with the correct name exists or specify a step name")
							.finish();
						error.prepend_object_index(pipe.port);
						input_errors.extend(error);
						continue;
					};

					let serde_json::Value::String(plot_input) = plot_input else {
						// Ignore invalid format here: the error is already reported.
						// (Same for the next continue statements.)
						continue;
					};
					let Some((input_type, _)) = plot_input.split_once(':') else {
						continue;
					};
					let allowed_types = FormulaValidator::allowed_input_types(pipe.port, true);
					if allowed_types.is_empty() {
						continue;
					}

					if !allowed_types.contains(&input_type) {
						let mut error =
							ValidationErrorWithPath::build("pipe type not allowed for port")
								.with_label("pipe of invalid type")
								.with_note(
									"change either port, pipe or type of referenced plot input",
								)
								.with_target(TargetHint::KeyAndValue)
								.finish();
						error.prepend_object_index(pipe.port);
						input_errors.extend(error);
					}
				}
			}

			input_errors.prepend_object_indices(&[target_step_name, "protoformula", "inputs"]);
			step_errors.extend(input_errors);
		}

		step_errors.prepend_object_index("steps");
		step_errors
	}

	fn check_graph_not_cyclic(&mut self, graph: &PlotGraph<'a>) -> Vec<ValidationErrorWithPath> {
		let mut parents = IndexMap::new();
		for (target_step_name, step) in &graph.steps {
			let mut step_inputs = IndexSet::new();
			for pipe in &step.input_pipes {
				if !pipe.step.is_empty() {
					step_inputs.insert(pipe.step);
				}
			}
			parents.insert(target_step_name, step_inputs);
		}

		Vec::with_capacity(0)
	}

	fn check_outputs(
		&self,
		value: &serde_json::Value,
		graph: &PlotGraph<'a>,
	) -> Vec<ValidationErrorWithPath> {
		let example_output = "example output: \"name\": \"pipe:step_name:step_output_name\"";
		expect_object_iterate(value, |(_key, value)| {
			expect_string(value, |value| {
				let mut parts = value.split(':');
				let Some("pipe") = parts.next() else {
					return ValidationErrorWithPath::build("plot output has to be pipe")
						.with_note(example_output)
						.finish();
				};

				let (Some(step), Some(output), None) = (parts.next(), parts.next(), parts.next())
				else {
					return ValidationErrorWithPath::build("expected step and output")
						.with_note(example_output)
						.finish();
				};

				if !step.is_empty() {
					let Some(graph_step) = graph.steps.get(step) else {
						return ValidationErrorWithPath::build("step does not exist")
							.with_label("invalid pipe")
							.finish();
					};

					if !graph_step.outputs.contains(&output) {
						let message = "target step does not contain specified output";
						return ValidationErrorWithPath::build(message)
							.with_label("invalid pipe")
							.finish();
					}
				}

				Vec::with_capacity(0)
			})
		})
	}
}
