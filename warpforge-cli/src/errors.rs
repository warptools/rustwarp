type ErrorCause = Box<dyn ::std::error::Error>; // `+ Send + Sync` has sometimes seemed virtuous here?  But it also produces conflicts.  Still don't understand what's wise here.

#[derive(thiserror::Error, Debug)]
pub enum Error {
	// InvalidArguments is for CLI level parse errors.  Don't use it any deeper inside.
	#[error("invalid arguments: {cause}")]
	InvalidArguments { cause: ErrorCause },

	/// BizarreEnvironment is a bit of a catch-all to describe...
	///   - missing environment variables that are VERY weird (like missing $HOME)
	///   - missing directories that are very weird (like missing /tmp)
	///   - ... anything we haven't figured out how to describe better yet.
	///
	/// In general it means "human intervention required", which is why we're okay with it being such a grab-bag.
	///
	/// Make sure the cause describes itself well, since this error's display message preamble provides little information itself.
	#[error("halting due to strange environment: {cause}")]
	BizarreEnvironment { cause: ErrorCause },

	/// MissingPlugin indicates that something is missing in the host environment that we need:
	/// typically it's another command that should be on $PATH or otherwise discoverable by us.
	/// (Don't use this for things like a missing kernel feature; that requires a bigger intervention to fix, so deserves a distinct error code.)
	#[error(
		"missing a plugin for {subsystem}: could not find or initialize {missing_plugin}: {cause}"
	)]
	MissingPlugin {
		/// The subsystem's descriptive name, e.g. "ware transport", "container engine", etc.
		subsystem: String,
		/// A more specific name of what exactly we're missing, e.g. "rio" or "runc", etc.
		missing_plugin: String,
		cause: ErrorCause,
	},

	// User-level "404"-like error.
	#[error("catalog entry doesn't exist -- there is no value referenced as {reference}")]
	CatalogEntryNotExists {
		reference: warpforge_api::catalog::CatalogRef,
	},

	/// Catch-all error for failing to look something up or write something in a catalog.
	/// Probably contains a filesystem IO error or similar.
	#[error("error accessing catalog: {cause}")]
	CatalogAccessError { cause: ErrorCause },
}

impl Error {
	pub fn code(&self) -> i32 {
		match self {
			Error::InvalidArguments { .. } => 1,
			Error::BizarreEnvironment { .. } => 4,
			Error::MissingPlugin { .. } => 7,
			Error::CatalogEntryNotExists { .. } => 14,
			Error::CatalogAccessError { .. } => 15,
		}
	}
}

// I'm tempted to write a macro that makes partial constructors for each of these,
// which take a parameter for everything _but_ the cause field.
// The result would be functions that are easy to use together with `Result::map_err`.
// ...
// Just kidding!  You can't associate methods with enum members.
// And it seems like thiserror doesn't jive with enum members being freestanding types.
//
// Next best thing?  You probably want to do something like this:
//    .map_err(|e| Error::WhichEverSpecificOne{foobar:"foo", cause:Box::new(e)})?;
// The closure is so syntactically compact that it's pretty hard to beat.
