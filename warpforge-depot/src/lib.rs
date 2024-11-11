use std::{error::Error, path::PathBuf};

use pathrs;
use warpforge_api as wfapi;

// rough plan:
// - there's three phases of operation (which have different relevant config options):
//   - list:
//     - fetching a packed ware
//     - maintaining the cache (of packed and unpacked/presentable stuff)
//     - getting a presentation dir (may involve unpacking, if not already present)
//   - sometimes only some of these are relevant
//     - ex: warpforge might've dropped a packed ware into that layer of cache, so it doesn't need to be fetched, but does still need to be unpacked for presentation before use.
// - any unpack function shall take a libpathrs root handle for where to emit.
//   - this then works with the NO_SYMLINKS flag already set.
//   - if the plugin works by exec'ing something, well, okay, I hope that is similarly secure; we can't do much from here.
//   - this almost always targets a tmpdir within the depot (that gets moved to CAS after unpacking is complete)...
//   - but sometimes it could be a user-provided path from a special CLI mode, instead.
// - at the end of an unpack op, that code returns a WareID.
//   - and we can write common code to assert it matches expectations, and do the mv of tmp dir into CAS.
//   - (for some pack systems, the wareID might be the hash of a packed stream we started with, but for some it requires a walk, so, doing it together with the walk of unpacking and returning the hash at the end is the way to go.)

fn _hello_pathrs() -> Result<(), pathrs::error::Error> {
	let _wow = pathrs::Root::open("/tmp/hewwo")?
		.with_resolver_flags(pathrs::flags::ResolverFlags::NO_SYMLINKS);
	Ok(())
}

pub struct Depot {
	root: pathrs::Root,
}

fn request_presentation(
	depot: &Depot,
	ware_id: &wfapi::content::WareID,
	opts_fetch: &FetchOptions,
	opts_cache: &CacheOptions,
	opts_presentation: &PresentationOptions,
) -> Result<Presentation, ()> {
	todo!()
}

pub enum Presentation {
	Path(PathBuf),
	MountSpec(/* FUTURE: support this? */),
}

pub struct FetchOptions {
	// URLs, mostly.  And auth.
	// Probably still bundling those into a concept called warehouse makes sense to me.  WarehouseDialConfig.
	//
	// Can also be blank, to say "don't" (only yield from cache if it's already here).
}

pub struct CacheOptions {
	// Mostly, whether to cache packed forms as well as presentable.
	// E.g., if we should keep the tgz file *as well as* unpacking it and keeping that.
	// This usually defaults to "on", so if you have an unpack cache mistrust event,
	//   you can feel free to blast the unpack cache without fearing subsequent big bandwidth needs.
	//
	// Unclear how abstract this can be.
	// Different pack systems have different granularity of cacheable component data.
}

pub struct PresentationOptions {
	// Future: can signal whether a mount is a desired option
	//   (for the rare pack system that supports both simple presentation and a mounted one).
	// Future: can ask to have a new presentation made in a target path
	//   (because that might be slightly cheaper than having it unpacked in the Depot, if it's not already).
}
