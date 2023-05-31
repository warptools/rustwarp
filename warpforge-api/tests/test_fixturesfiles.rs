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
use std::io;
use std::path::Path;
use testfiles_derive::test_per_file;

// Okay the amazing thing is, it works.
// The mild bummer thing is, I think VSCode's integrations making a "Run Test" button are going for a specific name, and filter it back out.  I don't know how others like `rstest` get around this (or if they're just blessed at this point).
#[test_per_file(glob = "fixtures/*.json")]
fn test_fixture(file_path: &Path) {
    print!("hello!  this is a test for {:?}\n", file_path)
}

// Roughly the same as `Result<(), String>`, if we're honest, but it's desirable to have domain-relevant names, and room to expand.
enum Expectation {
    Success(),
    ShouldError { display: String },
}

// Fixture files for the API have a simple format:
// the serial document starts right away, with no preamble;
// at some point, there's a "---" on a line by itself;
// and expectation information is below that.
//
// The aim of this format is to make it very easy to see line numbers in the main document.
// (We're interested in writing tests that include making sure parse *errors* from that
// document have good line and column offset info.  Making that easy to eyeball helps!)
//
// In this function, we don't have any opinion of what the main document is;
// we're just going to return bytes for both that, and the second half.
fn parse_API_fixture_file(fixture_path: &str) -> Result<(), String> {
    let file_content = fs::read(fixture_path).map_err(|e| e.to_string())?;
    let separator = b"\n---\n";
    let separator_position = memmem::find(&file_content, separator).ok_or_else(|| {
        "Separator not found (hint: '---' should be present, in one line, alone)".to_string()
    })?;
    let json_data = &file_content[..separator_position];
    let expected_result =
        String::from_utf8_lossy(&file_content[separator_position + separator.len()..])
            .trim()
            .to_string();

    let parsed_data: serde_json::Value =
        serde_json::from_slice(json_data).map_err(|e| e.to_string())?;

    if expected_result == "success" {
        Ok(())
    } else {
        Err(expected_result)
    }
}

// WIP: replacing the above with a more general thing, that splits $N times and doesn't do json yet.
// And I'm making it _very_ hard on myself by trying to go for zero-copy references to subslices.

struct Hunks<'a> {
    unsplit: Vec<u8>,
    split: Vec<&'a [u8]>,
}

// FIXME still trying to win an argument about ownership.  pushing the limits of my rust knowledge here.
//
// The return type is `Vec<&[u8]>` because the outer vec is a list we newly allocate,
// while the contents are references to slices of another slice of bytes (to avoid copying).
// (Any time you go from `&[T]` to `Vec<T>`, you've taken ownership of something you didn't previously own,
// and that means an allocation had to occur somewhere (even if an implicit clone might have hidden it).)
fn load_file_split_by_delimiter(filename: &Path) -> io::Result<Hunks> {
    let file_content = fs::read(filename)?;
    let mut hunks = Hunks {
        unsplit: file_content, // Hoped this would do a "move" that lets me grab slices and let them have a lifetime determined by the hunks value (and move with it) but apparently that's not how it works :(
        split: Vec::new(),
    };
    hunks.split = split_by_delimiter(&(hunks.unsplit.as_slice()));
    return Ok(hunks);
}

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
