#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("failed to parse {type_name} value: \"{value}\" is not a recognized discriminant")]
	// (accepted values are {accepted:?})
	UnknownDiscriminant {
		type_name: String,
		value: String,
		//   accepted: Vec<String>,
	},
}
