use std::{
	fmt::{Display, Formatter},
	ops::Range,
};

use json_with_position::{JsonPath, TargetHint};

use crate::common::find_byte_offset;

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

	pub(crate) fn try_set_span(&mut self, span: Range<usize>) -> bool {
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

pub(crate) struct ValidationErrorWithPath {
	pub(crate) path: JsonPath,
	pub(crate) target: TargetHint,
	pub(crate) inner: ValidationError,
}

impl ValidationErrorWithPath {
	pub(crate) fn build(message: impl Into<String>) -> PathErrorBuilder {
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

	pub(crate) fn custom(message: impl Into<String>) -> Vec<Self> {
		Self::build(message).finish()
	}
}

pub(crate) struct PathErrorBuilder {
	error: CustomError,
	target: TargetHint,
}

impl PathErrorBuilder {
	pub(crate) fn with_target(mut self, target: TargetHint) -> Self {
		self.target = target;
		self
	}

	pub(crate) fn with_note(mut self, note: impl Into<String>) -> Self {
		self.error.note = note.into();
		self
	}

	pub(crate) fn with_label(mut self, label: impl Into<String>) -> Self {
		self.error.label = label.into();
		self
	}

	pub(crate) fn finish(self) -> Vec<ValidationErrorWithPath> {
		vec![ValidationErrorWithPath {
			path: JsonPath::new(),
			target: self.target,
			inner: ValidationError::Custom(self.error),
		}]
	}
}
