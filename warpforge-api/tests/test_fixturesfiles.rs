/*
Notes on ways to handle fixture testing:

- rust out-of-box does not support dynamic test names.
	- And they have the typical (pretty valid) reasoning of wanting to be able to enumerate test names statically.
		- There's a discussion on https://internals.rust-lang.org/t/dynamic-tests-revisited/18095 but so far it lands somewhere pretty conservative.
	- But of course, this is rust, so we also have macros.  Which means Anything Is Possible.

Okay so who can help?

- rstest looks glorious in a lot of ways, but...
	- I haven't found an incantation that lets it generate a whole bunch of cases at once, yet.
- https://docs.rs/datatest/latest/datatest/ is neat and looks close to the mark...
	- and it even explicitly has a bunch of filesystem features built right in!  Nice!
	- except it requires a test runner, which is an odd requirement?  I think this should be possible with just proc macros.
		- Depending on `#![feature(custom_test_frameworks)]` seems like its entering deep water.
			- I'm really trying hard not to have to opt in to nightly compilers.
	- I have not yet analyzed if this has a fixture regen features.
- ...That's all I could find.

Are we gonna just do this again ourselves?  Ugh.  Apparently, yes.

"testfiles-derive" crate, here we come.

*/

use memchr::memmem;
use std::fs;
use std::path::Path;
use testfiles_derive::test_per_file;

// Fixture files for the API have a simple format:
// the serial document starts right away, with no preamble;
// at some point, there's a "---" on a line by itself;
// and expectation information is below that.
//
// The aim of this format is to make it very easy to see line numbers in the main document.
// (We're interested in writing tests that include making sure parse *errors* from that
// document have good line and column offset info.  Making that easy to eyeball helps!)
//
// We made our own test_per_file macro to power this.
// Okay the amazing thing is, it works.
// The mild bummer thing is, I think VSCode's integrations making a "Run Test" button are going for a specific name, and filter it back out.  I don't know how others like `rstest` get around this (or if they're just blessed at this point).
// Ah, yes, and of course the murderously bad thing is... cargo test caching is too smart.
#[test_per_file(glob = "fixtures/workflow_*.json")]
fn test_fixture(file_path: &Path) {
	let content = fs::read(file_path).unwrap();
	let hunks = split_by_delimiter(content.as_slice());

	let result: Result<warpforge_api::compute::Workflow, _> = serde_json::from_slice(&hunks[0]);
	match result {
		Ok(value) => {
			assert_eq!(std::str::from_utf8(&hunks[1]).unwrap(), "success\n");

			let reserialized = serde_json::to_string(&value).unwrap();
			let foobar: serde_json::Value = serde_json::from_slice(&hunks[0]).unwrap();
			let normalized = serde_json::to_string(&foobar).unwrap();
			assert_eq!(reserialized, normalized);
		}
		Err(err) => {
			assert_eq!(
				std::str::from_utf8(&hunks[1]).unwrap(),
				format!("{}\n", err)
			);
		}
	}
}

// Split the given slice by occurances of a magic delimiter ("\n---\n"),
// and return subslices of that slice.
//
// The return type is `Vec<&[u8]>` because the outer vec is a list we newly allocate,
// while the contents are references to slices within the original slice of bytes (to avoid copying).
// (Any time you go from `&[T]` to `Vec<T>`, you've taken ownership of something you didn't previously own,
// and that means an allocation had to occur somewhere (even if an implicit clone might have hidden it).
// That means if you tried to make this function as `Vec<u8> -> Vec<Vec<u8>>`, you'd have a copy-monster!)
fn split_by_delimiter<'a>(splitme: &'a [u8]) -> Vec<&'a [u8]> {
	let separator_bytes = b"\n---\n";
	let mut result = Vec::new();
	let mut current_chunk_start = 0;

	while let Some(separator_start) = memmem::find(&splitme[current_chunk_start..], separator_bytes)
	{
		result.push(&splitme[current_chunk_start..separator_start]);
		current_chunk_start = separator_start + separator_bytes.len();
	}

	// Push the last chunk if the file doesn't end with the separator marker
	if current_chunk_start < splitme.len() {
		result.push(&splitme[current_chunk_start..]);
	}

	return result;
}
