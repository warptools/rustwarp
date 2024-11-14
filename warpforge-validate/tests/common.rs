use std::{collections::HashMap, ops::Range};

use warpforge_validate::{Error, Result};

#[derive(Debug)]
enum InputToken<'a> {
	Slice(&'a str),
	Start(&'a str),
	End(&'a str),
	Eof,
}

#[test]
fn test_prepare_input() {
	// Doc-tests of integration tests are not run, so we added them as their own test.
	let input = "hello <tag>world <3</tag>";
	let (stripped, locations) = prepare_input(input);
	assert_eq!(&stripped, "hello world <3");
	assert_eq!(locations, vec![6..14]);
}

/// Takes tagged input and returns string stripped of tags and locations of the tags.
///
/// With this function we attempt to make test more readable and maintainable.
/// Instead of having to hardcode locations into the code we mark the locations with
/// tags and let [`prepare_input`] strip the tags and find the corresponding locations.
///
/// # Examples
///
/// ```
/// let input = "hello <tag>world <3</tag>";
/// let (stripped, locations) = prepare_input(input);
/// assert_eq!(&stripped, "hello world <3");
/// assert_eq!(locations, vec![6..14]);
/// ```
pub fn prepare_input(input: &str) -> (String, Vec<Range<usize>>) {
	let parsed = parse_input(input);

	let mut source = String::new();
	let mut locations = Vec::new();

	let mut location = 0;
	for i in 0..parsed.len() {
		let token = &parsed[i];
		match token {
			InputToken::Slice(slice) => {
				source.push_str(slice);
				location += slice.len();
			}
			InputToken::Start(tag) => {
				let mut end_location = location;
				for search_token in &parsed[(i + 1)..] {
					match search_token {
						InputToken::Slice(slice) => end_location += slice.len(),
						InputToken::End(end_tag) if end_tag == tag => {
							locations.push(location..end_location);
							break;
						}
						InputToken::Eof => panic!("missing closing tag for '<{tag}>' (add '</{tag}>' or remove '<{tag}>')"),
						_ => {}
					}
				}
			}
			_ => {}
		}
	}

	eprintln!("expected error locations:");
	for location in &locations {
		eprintln!("  - {location:?}");
	}
	eprintln!();

	(source, locations)
}

fn parse_input(mut input: &str) -> Vec<InputToken<'_>> {
	let mut parsed = Vec::new();

	while let Some(start) = input.find('<') {
		let Some(open_end) = input[start + 1..].find('>') else {
			let (left, right) = input.split_at(start + 1);
			parsed.push(InputToken::Slice(left));
			input = right;
			continue;
		};
		let end = (start + 1) + open_end;

		let mut tag = &input[start + 1..end];
		let is_end = tag.starts_with('/');
		if is_end {
			tag = &tag[1..];
		}
		if tag.chars().any(|c| !c.is_ascii_alphanumeric()) {
			let (left, right) = input.split_at(start + 1);
			parsed.push(InputToken::Slice(left));
			input = right;
			continue;
		}

		let (left, _) = input.split_at(start);
		(_, input) = input.split_at(end + 1);
		parsed.push(InputToken::Slice(left));

		let token = if is_end {
			InputToken::End(tag)
		} else {
			InputToken::Start(tag)
		};
		parsed.push(token);
	}

	parsed.push(InputToken::Slice(input));
	parsed.push(InputToken::Eof);

	parsed
}

pub fn check_validation_locations<T>(
	report: &Result<T>,
	source: &str,
	byte_locations: &[Range<usize>],
) {
	if byte_locations.is_empty() {
		assert!(report.is_ok());
		return;
	}

	let Err(err @ Error::Invalid { errors }) = report else {
		panic!("expected {} validation errors", byte_locations.len());
	};
	eprintln!("actual errors: {err}\n");

	let msg = "number of actual errors == number of expected errors";
	assert_eq!(errors.len(), byte_locations.len(), "{msg}");

	let mut lookup = HashMap::new();
	for location in byte_locations {
		lookup
			.entry(location)
			.and_modify(|entry| *entry += 1)
			.or_insert(1);
	}

	for err in errors {
		let actual = err.span(source).unwrap();
		let Some(entry) = lookup.get_mut(&actual) else {
			panic!("unexpected validation error at {actual:?}: {err}");
		};

		if *entry > 0 {
			*entry -= 1;
		} else {
			panic!("too many validation errors reported at {actual:?}");
		}
	}

	for (location, remain) in lookup {
		if remain > 0 {
			panic!("expected error at {location:?}"); // unreachable at time of writing
		}
	}
}
