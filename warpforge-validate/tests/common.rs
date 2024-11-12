use std::{collections::HashSet, ops::Range};

use warpforge_validate::{Error, Result};

pub fn prepare_input(mut input: &str) -> (String, Vec<Range<usize>>) {
	let mut source = String::new();
	let mut locations = Vec::new();

	while let Some(open_start) = input.find('<') {
		let Some(open_end) = input[open_start + 1..].find('>') else {
			source.push_str(&input[..open_start + 1]);
			input = &input[open_start + 1..];
			continue;
		};
		let open_end = (open_start + 1) + open_end;

		let tag = &input[open_start + 1..open_end];
		if tag.chars().any(|c| !c.is_ascii_alphanumeric()) {
			source.push_str(&input[..open_start + 1]);
			input = &input[open_start + 1..];
			continue;
		}

		let close_tag = format!("</{tag}>");
		let Some(close_start) = input.find(&close_tag) else {
			panic!("missing closing tag for '<{tag}>' (add '{close_tag}' or remove '<{tag}>')");
		};

		let location_start = source.len() + open_start;
		locations.push(location_start..(location_start + (close_start - open_end - 1)));
		source.push_str(&input[..open_start]);
		source.push_str(&input[(open_end + 1)..close_start]);
		input = &input[close_start + close_tag.len()..];
	}

	source.push_str(input);

	eprintln!("expected error locations:");
	for location in &locations {
		eprintln!("  - {location:?}");
	}

	(source, locations)
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
	eprintln!("{err}");
	assert_eq!(errors.len(), byte_locations.len());

	let mut lookup: HashSet<_> = byte_locations.iter().collect();
	let msg = "multiple identical locations are currently not supported by the testing framework";
	assert_eq!(lookup.len(), byte_locations.len(), "{msg}");

	for err in errors {
		let actual = err.span(source).unwrap();
		assert!(
			lookup.remove(&actual),
			"unexpected error at {actual:?}: {err}"
		)
	}
}
