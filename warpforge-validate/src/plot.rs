use json_with_position::TargetHint;
use oci_client::Reference;

use crate::{
	common::{expect_key, expect_object_iterate, expect_string},
	error::ValidationErrorWithPath,
	formula::FormulaValidator,
};

pub(crate) struct PlotValidator {}

impl PlotValidator {
	pub(crate) fn validate(parsed: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		PlotValidator {}.check(parsed)
	}

	fn check(&mut self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		expect_key(value, "plot.v1", |value| {
			let mut errors = expect_key(value, "inputs", |value| self.check_inputs(value));
			errors.extend(expect_key(value, "steps", |value| self.check_steps(value)));
			errors.extend(expect_key(value, "outputs", |value| {
				self.check_outputs(value)
			}));

			errors
		})
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

	fn check_steps(&self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		expect_object_iterate(value, |(key, value)| {
			let mut errors = expect_key(value, "protoformula", |value| {
				FormulaValidator::validate(value, true)
			});

			if key.is_empty() {
				let key_error = ValidationErrorWithPath::build("empy step name not allowed")
					.with_label("specify step name here")
					.with_target(TargetHint::Key)
					.finish();
				errors.extend(key_error);
			}

			errors
		})
	}

	fn check_outputs(&self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		let example_output = "example output: \"name\": \"pipe:step_name:step_output_name\"";
		expect_object_iterate(value, |(_key, value)| {
			expect_string(value, |value| {
				let mut parts = value.split(':');
				let Some("pipe") = parts.next() else {
					return ValidationErrorWithPath::build("plot output has to be pipe")
						.with_note(example_output)
						.finish();
				};

				let (Some(step), Some(output)) = (parts.next(), parts.next()) else {
					return ValidationErrorWithPath::build("expected step and output")
						.with_note(example_output)
						.finish();
				};

				// TODO: Check if step and output exist

				Vec::with_capacity(0)
			})
		})
	}
}
