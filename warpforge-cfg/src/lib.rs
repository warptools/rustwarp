use std::path::PathBuf;
use xdg;

// Future work: *much*.
// - workspace detection logic should be gathered here.
// - most of these functions should move to methods on a struct that is created by doing workspace detection.
// - that struct does also have to deal with the situation of no workspace being detected.

/// Returns a path where warpforge will maintain the user's global default depot.
/// The default is to take a directory within XDG_CACHE_DIR.
/// (Depot data is considered "cache", semantically, because it's considered locally managed,
/// and ephemeral.  It does not make sense to version control Depot data,
/// nor typically to include it in the reach of any automatic synchronization systems
/// (as it can be sizable).)
///
/// It is also possible for a workspace to specify its own depot path,
/// in which case this path is not used.
pub fn get_depot_dir() -> Result<PathBuf, xdg::BaseDirectoriesError> {
	// Quick little note: XDG_STATE_HOME or XDG_CACHE_HOME -- which is more appropriate?
	// It's hard to say; a "six of one, half a dozen of the other" situation.
	// The XDG spec gives these examples for state: "logs, history, recently used files" and "view, layout, open files, undo history".
	// Cache seems like a slightly better description than state, because essentially all contents of a depot --
	// both the packed and unpacked files -- are intended to be regeneratable.
	let dirs = xdg::BaseDirectories::new()?;
	return Ok(dirs.get_cache_home().join("warpforge/depot"));
}

/// Returns a path where warpforge will create container root dirs and perform mount assemblies.
/// The default is to take a directory within XDG_RUNTIME_DIR,
/// since such path will have permissions of 0700 and be owned by the current user.
///
/// Typically, this will come out to something like "/run/user/{uid}/warpforge/ersatz/".
pub fn get_container_ersatz_basedir() -> Result<PathBuf, xdg::BaseDirectoriesError> {
	let dirs = xdg::BaseDirectories::new()?;
	return Ok(dirs.get_runtime_directory()?.join("warpforge/ersatz"));
}
