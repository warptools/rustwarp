type ErrorCause = Box<dyn ::std::error::Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("{msg}: {cause}")]
	Catchall {
		msg: String, //
		cause: ErrorCause,
	},
}
