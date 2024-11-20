mod common;
mod error;
mod formula;
mod plot;

use std::mem;

use warpforge_api::{formula::FormulaAndContext, plot::PlotCapsule};
use warpforge_terminal::{debug, warn};

use crate::common::find_byte_offset;
use crate::error::ValidationErrorWithPath;
pub use crate::error::{CustomError, Error, Result, TrailingComma, ValidationError};
use crate::formula::FormulaValidator;

/// Maximal number of trailing comma errors that we include in validation result.
const MAX_TRAILING_COMMA: usize = 20;

pub fn validate_formula(formula: &str) -> Result<ValidatedFormula> {
	// Documentation from serde_json::from_reader about performance:
	// "Note that counter to intuition, this function (from_reader) is usually
	// slower than reading a file completely into memory and then applying
	// `from_str` or `from_slice` on it. See [issue #160]."
	// [issue #160]: https://github.com/serde-rs/json/issues/160

	let mut parser = Parser::parse_json_value(formula)?;
	let errors = FormulaValidator::validate(&parser.parsed, false);
	parser.append_errors(errors);
	parser.finish_formula()
}

pub fn validate_plot(plot: &str) -> Result<ValidatedPlot> {
	todo!()
}

pub struct ValidatedFormula {
	pub formula: FormulaAndContext,
}

pub struct ValidatedPlot {
	pub plot: PlotCapsule,
}

struct Parser<'a> {
	modified_json: Option<Vec<u8>>,
	errors: Vec<ValidationError>,
	json: &'a str,
	parsed: serde_json::Value,
}

impl<'a> Parser<'a> {
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

	fn append_errors(&mut self, errors: Vec<ValidationErrorWithPath>) {
		if errors.is_empty() {
			return;
		}

		let json = (self.modified_json.as_deref()).unwrap_or(self.json.as_bytes());
		let Ok(positions) = json_with_position::from_slice(json) else {
			debug!("failed to get position of some errors");
			self.errors
				.extend(errors.into_iter().map(|path_err| path_err.inner));
			return;
		};

		for mut error in errors {
			let Some(span) = positions.find_span(&error.path, error.target) else {
				continue;
			};
			error.inner.try_set_span(span);
			self.errors.push(error.inner);
		}
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
				if self.errors.is_empty() {
					// We assume here that the reported errors cover the error found by serde.
					// Note: When finding syntax errors other than trailing comma we exit early.
					self.errors.push(ValidationError::Serde(err));
				}
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
}

fn err_is_trailing_comma(err: &serde_json::Error) -> bool {
	// serde_json provides no better way to branch on a concrete error type.
	err.is_syntax() && format!("{err}").starts_with("trailing comma")
}
