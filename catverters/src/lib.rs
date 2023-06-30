#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("failed to parse {type_name} value: \"{value}\" is not a recognized discriminant")]
	// (accepted values are {accepted:?})
	UnknownDiscriminant {
		type_name: String,
		value: String,
		//   accepted: Vec<String>,
	},

	#[error("failed to parse {type_name} value: \"{value}\" is missing a separator (should contain \"{expected_separator}\")")]
	MissingSeparator {
		type_name: String,
		value: String,
		expected_separator: String,
	},

	#[error("failed to parse {type_name} value: \"{value}\" should have more separators (the next one would delimit the start of the {next_field_name} field; the separator is \"{expected_separator}\")")]
	InsufficientHunks {
		type_name: String,
		value: String,
		expected_separator: String,
		next_field_name: String,
	},

	#[error("failed to parse {type_name} value: field {problem_field_name} reported an error: {cause}")]
	FieldParseFailure {
		type_name: String,
		problem_field_name: String,
		cause: Box<dyn ::std::error::Error + Send + Sync>,
	},
}
