pub type Result<T> = std::result::Result<T, Error>;

type ErrorCause = Box<dyn ::std::error::Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	/// SystemRuntimeError should be used for unexpected failures of things at runtime.
	/// Examples include spawning processes (failing because you're out of PIDs, etc)
	/// or failing to set up IO (out of fd's, etc) -- things expected to be transient.
	#[error("{msg}: {cause}")]
	SystemRuntimeError {
		msg: String,
		#[source]
		cause: ErrorCause,
		//backtrace: std::backtrace::Backtrace, // doesn't work, reason unknown.
	},

	/// SystemSetupError is for failures during execution that are probably going to
	/// fail repeatedly until a human intervenes.
	/// Examples include permission errors while writing container setup files, etc.
	#[error("{msg}: {cause}")]
	SystemSetupError {
		msg: String,
		#[source]
		cause: ErrorCause,
	},

	#[error("{msg}")]
	SystemSetupCauseless { msg: String },

	#[error("{msg}: {cause}")]
	Catchall { msg: String, cause: ErrorCause },

	#[error("{msg}")]
	CatchallCauseless { msg: String },
}
