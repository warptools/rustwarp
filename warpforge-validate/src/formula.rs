use json_with_position::TargetHint;
use oci_client::Reference;

use crate::{
	common::{
		accept_any, expect_array_iterate, expect_key, expect_object_iterate, expect_string,
		optional_key,
	},
	error::ValidationErrorWithPath,
};

pub(crate) struct FormulaValidator {
	protoformula: bool,
}

impl FormulaValidator {
	pub(crate) fn validate(
		parsed: &serde_json::Value,
		protoformula: bool,
	) -> Vec<ValidationErrorWithPath> {
		Self { protoformula }.check(parsed)
	}

	pub(crate) fn new(protoformula: bool) -> Self {
		Self { protoformula }
	}

	pub(crate) fn check(&mut self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		if self.protoformula {
			self.check_formula_entries(value)
		} else {
			expect_key(value, "formula", |value| {
				expect_key(value, "formula.v1", |value| {
					self.check_formula_entries(value)
				})
			})
		}
	}

	pub(crate) fn allowed_input_types(port: &str, protoformula: bool) -> &'static [&'static str] {
		match (port.get(..1), protoformula) {
			(Some("/"), false) => &["mount", "ware"][..],
			(Some("/"), true) => &["mount", "ware", "pipe"][..],
			(Some("$"), false) => &["literal"][..],
			(Some("$"), true) => &["literal", "pipe"][..],
			_ => &[][..],
		}
	}

	fn check_formula_entries(&mut self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		let mut errors = expect_key(value, "inputs", |value| self.check_inputs(value));
		errors.extend(expect_key(value, "action", |value| {
			self.check_action(value)
		}));
		errors.extend(expect_key(value, "outputs", |value| {
			self.check_outputs(value)
		}));

		errors
	}

	fn check_inputs(&mut self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		let mut errors = expect_key(value, "/", |value| {
			expect_string(value, |value| {
				// TODO: allow pipe input for "/" if pipe points to an oci input. (protoformula only)
				let Some(oci) = value.strip_prefix("oci:") else {
					return ValidationErrorWithPath::custom(
						"formula input '/' currently has to be of type 'oci'",
					);
				};

				let reference = match oci.parse::<Reference>() {
					Ok(reference) => reference,
					Err(err) => {
						return ValidationErrorWithPath::custom(format!(
							"failed to parse oci reference: {err}"
						));
					}
				};

				if !self.protoformula && reference.digest().is_none() {
					return ValidationErrorWithPath::build(
						"formula inputs of type 'oci' are required to contain digest",
					)
					.with_label("invalid oci reference")
					.with_note("use '@' to add a digest: \"oci:docker.io/library/busybox@sha256:<DIGEST>\"")
					.finish();
				}

				Vec::with_capacity(0)
			})
		});

		errors.extend(expect_object_iterate(value, |(key, value)| {
			if key == "/" {
				return Vec::with_capacity(0);
			}

			let allowed_types = Self::allowed_input_types(key, self.protoformula);
			if allowed_types.is_empty() {
				return ValidationErrorWithPath::build("input port should start with '/' or '$'")
					.with_target(TargetHint::Key)
					.with_label("invalid port")
					.with_note(
						"use '/some/path' to mount an input or '$VAR' to set an env variable.",
					)
					.finish();
			}

			expect_string(value, |value| self.check_input_value(value, allowed_types))
		}));

		errors
	}

	fn check_input_value(
		&mut self,
		value: &str,
		allowed_types: &[&str],
	) -> Vec<ValidationErrorWithPath> {
		let mut value = value.split(':');
		let discriminant = value.next().expect("split emits at least one value");

		if !allowed_types.contains(&discriminant) {
			let message = format!(
				"input type not allowed (allowed types: '{}')",
				allowed_types.join("', '")
			);
			return ValidationErrorWithPath::build(message)
				.with_label("invalid formula input")
				.finish();
		}

		match discriminant {
			"pipe" => {} // Pipes are checked in the plot validation code.
			"literal" => {
				if value.next().is_none() {
					return ValidationErrorWithPath::build("input type 'literal' requires value")
						.with_label("invalid literal")
						.with_note("example input: \"$MSG\": \"literal:Hello, World!\"")
						.finish();
				}
			}
			"mount" => {
				let (Some(mount_type), Some(_host_path)) = (value.next(), value.next()) else {
					return ValidationErrorWithPath::build(
						"input type 'mount' requires mount type and host path",
					)
					.with_label("invalid mount")
					.with_note("example mount: \"/guest/path\": \"mount:ro:/host/path\"")
					.finish();
				};

				if !["ro", "rw", "overlay"].contains(&mount_type) {
					return ValidationErrorWithPath::build(
						"mount type not allowed (allowed types: 'ro', 'rw', 'overlay')",
					)
					.with_label("mount with invalid mount type")
					.with_note("example mount: \"/guest/path\": \"mount:ro:/host/path\"")
					.finish();
				}
			}
			"ware" => {
				todo!();
			}
			_ => {}
		}

		Vec::with_capacity(0)
	}

	fn check_action(&mut self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		if let Some("echo") = value.as_str() {
			return Vec::with_capacity(0);
		}

		let mut errors = expect_object_iterate(value, |(key, value)| match key.as_str() {
			"exec" => expect_key(value, "command", |value| {
				expect_array_iterate(value, |value| expect_string(value, accept_any))
			}),
			"script" => {
				let mut errors = expect_key(value, "interpreter", |value| {
					expect_string(value, |value| {
						if !value.starts_with("/") {
							ValidationErrorWithPath::custom("interpreter has to be absolute path")
						} else {
							Vec::with_capacity(0)
						}
					})
				});

				errors.extend(expect_key(value, "contents", |value| {
					expect_array_iterate(value, |value| expect_string(value, accept_any))
				}));

				errors
			}
			_invalid_action => {
				ValidationErrorWithPath::build("invalid action (allowed actions: 'exec', 'script')")
					.with_target(TargetHint::Key)
					.finish()
			}
		});

		if let Some(object) = value.as_object() {
			if object.len() != 1 {
				errors.extend(
					ValidationErrorWithPath::build(
						"a formula should define one action (allowed actions: 'exec', 'script')",
					)
					.with_note("example action: \"action\": { \"exec\": { \"command\": [\"echo\", \"hello, warpforge\"] } }")
					.finish(),
				);
			}
		}

		errors
	}

	fn check_outputs(&mut self, value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
		expect_object_iterate(value, |(_key, value)| {
			// TODO: check key not empty
			let mut errors = expect_key(value, "from", |value| {
				expect_string(value, |value| {
					if !value.starts_with('/') {
						return ValidationErrorWithPath::custom("expected an absolute path");
					}
					Vec::with_capacity(0)
				})
			});

			errors.extend(optional_key(value, "packtype", |value| {
				expect_string(value, |value| {
					if !["none", "tgz"].contains(&value) {
						let message = "invalid packtype (allowed values: 'none', 'tgz')";
						return ValidationErrorWithPath::custom(message);
					}
					Vec::with_capacity(0)
				})
			}));

			errors
		})
	}
}
