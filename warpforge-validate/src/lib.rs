use std::{
	fmt::{Display, Formatter},
	mem,
	ops::Range,
};

use json_with_position::{JsonPath, PathPart, TargetHint};
use oci_client::Reference;
use warpforge_api::formula::{FormulaAndContext, FormulaInput};
use warpforge_terminal::{debug, warn};

/// Maximal number of trailing comma errors that we include in validation result.
const MAX_TRAILING_COMMA: usize = 20;

pub fn validate_formula(formula: &str) -> Result<ValidatedFormula> {
	// Documentation from serde_json::from_reader about performance:
	// "Note that counter to intuition, this function (from_reader) is usually
	// slower than reading a file completely into memory and then applying
	// `from_str` or `from_slice` on it. See [issue #160]."
	// [issue #160]: https://github.com/serde-rs/json/issues/160

	let mut validator = Validator::parse_json_value(formula)?;
	validator.validate_formula()?;
	validator.finish_formula()
}

pub struct ValidatedFormula {
	pub formula: FormulaAndContext,
}

struct Validator<'a> {
	modified_json: Option<Vec<u8>>,
	errors: Vec<ValidationError>,
	json: &'a str,
	parsed: serde_json::Value,
}

impl<'a> Validator<'a> {
	fn parse_json_value(json: &'a str) -> Result<Self> {
		let mut modified_json = None;

		// We parse to `serde_json::Value` because we want to be able to generate
		// multiple erros if present: When deserializing to a struct, serde_json
		// fails fast and only reports the first error. For users this can lead to
		// a tedious bug chasing, where they 1st fix one thing, 2nd rerun, 3rd get
		// the next error. Instead we want to show all errors we can find at once.
		let parsed = serde_json::from_str::<serde_json::Value>(json);

		// Handle json syntax errors.
		let (parsed, errors) = match parsed {
			Ok(parsed) => (parsed, Vec::with_capacity(0)),
			Err(mut err) => {
				// Replacing trailing commas with white space is an easy fix,
				// which always works. We do this to be able to continue parsing
				// and find as many errors as possible.
				let mut errors = Vec::new();
				loop {
					if !err_is_trailing_comma(&err) {
						errors.push(ValidationError::Serde(err));
						return Err(Error::Invalid { errors });
					} else {
						let (line, column) = (err.line(), err.column());
						errors.push(ValidationError::Serde(err));

						let Some(mut offset) = find_byte_offset(json.as_bytes(), line, column)
						else {
							warn!("trailing comma error, but could not find comma");
							return Err(Error::Invalid { errors });
						};

						// We only create the vector on the error path to avoid allocations on the hot path.
						modified_json =
							Some(modified_json.unwrap_or_else(|| json.as_bytes().to_owned()));
						let modified_json = modified_json.as_mut().unwrap();

						// Find trailing comma, since serde_json points to closing braces instead of comma.
						// `serde_json::Error` does not allow us to match the concrete error kind,
						// so we look at the emitted error message.
						while offset > 0 {
							offset -= 1;
							if modified_json[offset] == b',' {
								break;
							} else if !modified_json[offset].is_ascii_whitespace() {
								warn!("trailing comma error, but could not find comma");
								return Err(Error::Invalid { errors });
							}
						}
						modified_json[offset] = b' ';

						let Some(ValidationError::Serde(serde_error)) = errors.pop() else {
							debug!("failed to pop value, we just pushed");
							return Err(Error::Invalid { errors });
						};
						errors.push(ValidationError::TrailingComma(TrailingComma {
							span: offset..(offset + 1),
							serde_error,
						}));

						if errors.len() >= MAX_TRAILING_COMMA {
							return Err(Error::Invalid { errors });
						}

						match serde_json::from_slice::<serde_json::Value>(modified_json) {
							// We only encountered trailing comma errors, we
							// continue validation to potentially find other errors.
							Ok(parsed) => break (parsed, errors),

							Err(next_err) => err = next_err,
						}
					}
				}
			}
		};

		Ok(Self {
			modified_json,
			errors,
			json,
			parsed,
		})
	}

	fn finish_formula(mut self) -> Result<ValidatedFormula> {
		let deserialize_err = if self.errors.is_empty() {
			// Setting self.parsed to Value::default here:
			// Don't use self.parsed anymore from here on.
			let parsed = mem::take(&mut self.parsed);
			match serde_json::from_value(parsed) {
				Ok(validated) => return Ok(ValidatedFormula { formula: validated }),
				Err(err) => Some(err),
			}
		} else {
			None
		};

		self.finish_error(deserialize_err)
	}

	fn finish_error<T>(mut self, deserialize_err: Option<serde_json::Error>) -> Result<T> {
		// Parse again with serde_json::from_slice to get line and column in error.
		// serde_json::from_value populates line and column with 0.
		let json = (self.modified_json.as_deref()).unwrap_or(self.json.as_bytes());
		let parse_result = serde_json::from_slice::<FormulaAndContext>(json);
		match (parse_result, deserialize_err) {
			(Err(err), _) => {
				self.errors.push(ValidationError::Serde(err));
			}
			(Ok(_), None) => {}
			(Ok(_), Some(err)) => {
				debug!("serde_json::from_value found error that serde_json::from_slice did not");
				self.errors.push(ValidationError::Serde(err));
			}
		}

		Err(Error::Invalid {
			errors: self.errors,
		})
	}

	fn validate_formula(&mut self) -> Result<()> {
		let errors = self.check_formula(&self.parsed, false);
		if errors.is_empty() {
			return Ok(());
		}

		let json = (self.modified_json.as_deref()).unwrap_or(self.json.as_bytes());
		let Ok(positions) = json_with_position::from_slice(json) else {
			debug!("failed to get position of some errors");
			self.errors
				.extend(errors.into_iter().map(|path_err| path_err.inner));
			return Ok(());
		};

		for mut error in errors {
			let Some(span) = positions.find_span(&error.path, error.target) else {
				continue;
			};
			error.inner.try_set_span(span);
			self.errors.push(error.inner);
		}

		Ok(())
	}

	fn check_formula(
		&self,
		value: &serde_json::Value,
		protoformula: bool,
	) -> Vec<ValidationErrorWithPath> {
		expect_key(value, "formula", |value| {
			expect_key(value, "formula.v1", |value| {
				let mut errors = expect_key(value, "inputs", |value| {
					self.check_formula_inputs(value, protoformula)
				});
				errors.append(&mut expect_key(value, "action", |value| {
					Vec::with_capacity(0) // TODO
				}));
				errors.append(&mut expect_key(value, "outputs", |value| {
					Vec::with_capacity(0) // TODO
				}));

				errors
			})
		})
	}

	fn check_formula_inputs(
		&self,
		value: &serde_json::Value,
		protoformula: bool,
	) -> Vec<ValidationErrorWithPath> {
		let mut errors = expect_key(value, "/", |value| {
			expect_string(value, |value| {
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

				if !protoformula && reference.digest().is_none() {
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

			let allowed_types = match key.get(..1) {
				Some("/") => &["mount", "ware"][..],
				Some("$") => &["literal"][..],
				_ => {
					return ValidationErrorWithPath::build(
						"input port should start with '/' or '$'",
					)
					.with_target(TargetHint::Key)
					.with_label("invalid port")
					.with_note(
						"use '/some/path' to mount an input or '$VAR' to set an env variable.",
					)
					.finish();
				}
			};

			expect_string(value, |value| {
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
					"literal" => {
						if value.next().is_none() {
							return ValidationErrorWithPath::build(
								"input type 'literal' requires value",
							)
							.with_label("invalid literal")
							.with_note("example input: \"$MSG\": \"literal:Hello, World!\"")
							.finish();
						}
					}
					"mount" => {
						let (Some(mount_type), Some(_host_path)) = (value.next(), value.next())
						else {
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
			})
		}));

		errors
	}
}

fn find_byte_offset(src: &[u8], line: usize, column: usize) -> Option<usize> {
	let mut walk_line = 1;
	let mut walk_column = 1;
	let mut offset = 0;
	while offset < src.len() && (walk_line < line || (walk_line == line && walk_column < column)) {
		if src[offset] == b'\n' {
			walk_line += 1;
			walk_column = 1;
		} else {
			walk_column += 1;
		}
		offset += 1;
	}

	if offset >= src.len() || walk_line != line || walk_column != column {
		None
	} else {
		Some(offset)
	}
}

fn err_is_trailing_comma(err: &serde_json::Error) -> bool {
	// serde_json provides no better way to branch on a concrete error type.
	err.is_syntax() && format!("{err}").starts_with("trailing comma")
}

#[must_use]
fn expect_key<'a>(
	value: &'a serde_json::Value,
	key: &str,
	inspect: impl FnOnce(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(target) = value.as_object().and_then(|object| object.get(key)) else {
		return ValidationErrorWithPath::custom(format!("missing field '{key}'"));
	};

	let mut errors = inspect(target);
	for error in &mut errors {
		error.path.prepend(PathPart::Object(key.to_owned()));
	}
	errors
}

#[must_use]
fn expect_index<'a>(
	value: &'a serde_json::Value,
	index: usize,
	inspect: impl FnOnce(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(target) = value.as_array().and_then(|vec| vec.get(index)) else {
		return ValidationErrorWithPath::custom(format!("missing entry at index '{index}'"));
	};

	let mut errors = inspect(target);
	for error in &mut errors {
		error.path.prepend(PathPart::Array(index));
	}
	errors
}

#[must_use]
fn expect_object_iterate<'a>(
	value: &'a serde_json::Value,
	mut inspect: impl FnMut((&'a String, &'a serde_json::Value)) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(object) = value.as_object() else {
		return ValidationErrorWithPath::custom("expected object");
	};

	let mut errors = Vec::with_capacity(0);
	for entry in object {
		for mut error in inspect(entry) {
			error.path.prepend(PathPart::Object(entry.0.to_owned()));
			errors.push(error);
		}
	}
	errors
}

#[must_use]
fn expect_array_iterate<'a>(
	value: &'a serde_json::Value,
	mut inspect: impl FnMut(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(array) = value.as_array() else {
		return ValidationErrorWithPath::custom("expected array");
	};

	let mut errors = Vec::with_capacity(0);
	for (index, entry) in array.iter().enumerate() {
		for mut error in inspect(entry) {
			error.path.prepend(PathPart::Array(index));
			errors.push(error);
		}
	}
	errors
}

#[must_use]
fn expect_string<'a>(
	value: &'a serde_json::Value,
	inspect: impl FnOnce(&'a str) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(string) = value.as_str() else {
		return ValidationErrorWithPath::custom("expected string");
	};
	inspect(string)
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Invalid { errors: Vec<ValidationError> },
}

#[derive(Debug)]
pub enum ValidationError {
	Serde(serde_json::Error),

	TrailingComma(TrailingComma),

	Custom(CustomError),
}

#[derive(Debug)]
pub struct TrailingComma {
	pub span: Range<usize>,
	pub serde_error: serde_json::Error,
}

#[derive(Debug, Default)]
pub struct CustomError {
	pub span: Range<usize>,
	pub message: String,
	pub note: String,
	pub label: String,
}

impl ValidationError {
	pub fn is_trailing_comma(&self) -> bool {
		matches!(self, ValidationError::TrailingComma(_))
	}

	pub fn span(&self, source: &str) -> Option<Range<usize>> {
		match self {
			ValidationError::Serde(err) => {
				find_byte_offset(source.as_bytes(), err.line(), err.column())
					.map(|offset| offset..offset)
			}
			ValidationError::TrailingComma(err) => Some(err.span.clone()),
			ValidationError::Custom(err) => Some(err.span.clone()),
		}
	}

	pub fn label(&self) -> Option<&str> {
		match self {
			ValidationError::Custom(err) if !err.label.is_empty() => Some(&err.label),
			_ => None,
		}
	}

	pub fn note(&self) -> Option<&str> {
		match self {
			ValidationError::Custom(err) if !err.note.is_empty() => Some(&err.note),
			_ => None,
		}
	}

	pub fn try_set_span(&mut self, span: Range<usize>) -> bool {
		match self {
			ValidationError::Serde(..) => {
				return false;
			}
			ValidationError::TrailingComma(err) => err.span = span,
			ValidationError::Custom(err) => err.span = span,
		}
		true
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "failed validation with error(s):")?;
		let Error::Invalid { errors } = self;
		for err in errors {
			write!(f, "\n  - {err}")?;
		}
		Ok(())
	}
}

impl Display for ValidationError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			ValidationError::Serde(err) => write!(f, "{err}"),
			ValidationError::TrailingComma(trailing_comma) => {
				write!(f, "{}", trailing_comma.serde_error)
			}
			ValidationError::Custom(custom_error) => write!(f, "{}", custom_error.message),
		}
	}
}

struct ValidationErrorWithPath {
	path: JsonPath,
	target: TargetHint,
	inner: ValidationError,
}

impl ValidationErrorWithPath {
	fn from_error(error: ValidationError) -> Vec<Self> {
		vec![Self {
			path: JsonPath::new(),
			target: TargetHint::Value,
			inner: error,
		}]
	}

	fn build(message: impl Into<String>) -> PathErrorBuilder {
		PathErrorBuilder {
			error: CustomError {
				message: message.into(),
				note: String::with_capacity(0),
				label: String::with_capacity(0),
				..Default::default()
			},
			target: TargetHint::Value,
		}
	}

	fn custom(message: impl Into<String>) -> Vec<Self> {
		Self::build(message).finish()
	}
}

struct PathErrorBuilder {
	error: CustomError,
	target: TargetHint,
}

impl PathErrorBuilder {
	fn with_target(mut self, target: TargetHint) -> Self {
		self.target = target;
		self
	}

	fn with_note(mut self, note: impl Into<String>) -> Self {
		self.error.note = note.into();
		self
	}

	fn with_label(mut self, label: impl Into<String>) -> Self {
		self.error.label = label.into();
		self
	}

	fn finish(self) -> Vec<ValidationErrorWithPath> {
		vec![ValidationErrorWithPath {
			path: JsonPath::new(),
			target: self.target,
			inner: ValidationError::Custom(self.error),
		}]
	}
}
