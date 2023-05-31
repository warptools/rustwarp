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

// use std::path::Path;
// use memchr::memmem;
use testfiles_derive::test_per_file;

// Okay the amazing thing is, it works.
// The mild bummer thing is, I think VSCode's integrations making a "Run Test" button are going for a specific name, and filter it back out.  I don't know how others like `rstest` get around this (or if they're just blessed at this point).
#[test_per_file(glob = "fixtures/*.json")]
fn test_fixture(file_path: &Path) {
    print!("hello!  this is a test for {:?}\n", file_path)
}

#[test]
fn please() {
    print!("this one is manual\n") // sanity checking that test filters work like i think they do
}

// Code below is playing with how to parse fixture files.  Not done yet.
// Key idea is that the document goes at the top of the file, without preamble,
// so that any line number info in error messages is easy to eyeball for correctness.
/*

// Roughly the same as `Result<(), String>`, if we're honest, but it's desirable to have domain-relevant names, and room to expand.
enum Expectation {
    Success(),
    ShouldError{display: String},
}

fn parse_json_fixture(fixture_path: &str) -> Result<(), String> {
    let file_content = fs::read(fixture_path).map_err(|e| e.to_string())?;
    let separator = b"\n---\n";
    let separator_position =
        memmem::find(&file_content, separator).ok_or_else(|| "Separator not found".to_string())?;
    let json_data = &file_content[..separator_position];
    let expected_result =
        String::from_utf8_lossy(&file_content[separator_position + separator.len()..])
            .trim()
            .to_string();

    let parsed_data: serde_json::Value = serde_json::from_slice(json_data).map_err(|e| e.to_string())?;

    if expected_result == "success" {
        Ok(())
    } else {
        Err(expected_result)
    }
}

*/
