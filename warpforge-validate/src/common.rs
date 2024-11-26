use crate::error::{ValidationErrorWithPath, VecValidationErrorWithPath};

pub(crate) fn find_byte_offset(src: &[u8], line: usize, column: usize) -> Option<usize> {
	let mut walk_line = 1;
	let mut walk_column = 1;
	let mut offset = 0;
	while offset < src.len() && (walk_line < line || (walk_line == line && walk_column < column)) {
		if src[offset] == b'\n' {
			walk_line += 1;
			walk_column = 1;
		} else {
			walk_column += 1;
		}
		offset += 1;
	}

	if offset >= src.len() || walk_line != line || walk_column != column {
		None
	} else {
		Some(offset)
	}
}

#[must_use]
pub(crate) fn expect_key<'a>(
	value: &'a serde_json::Value,
	key: &str,
	inspect: impl FnOnce(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(target) = value.as_object().and_then(|object| object.get(key)) else {
		return ValidationErrorWithPath::custom(format!("missing field '{key}'"));
	};

	let mut errors = inspect(target);
	errors.prepend_object_index(key);
	errors
}

#[must_use]
pub(crate) fn optional_key<'a>(
	value: &'a serde_json::Value,
	key: &str,
	inspect: impl FnOnce(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(object) = value.as_object() else {
		return ValidationErrorWithPath::custom("expected object");
	};

	let Some(target) = object.get(key) else {
		return Vec::with_capacity(0);
	};

	let mut errors = inspect(target);
	errors.prepend_object_index(key);
	errors
}

#[must_use]
pub(crate) fn expect_index<'a>(
	value: &'a serde_json::Value,
	index: usize,
	inspect: impl FnOnce(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(target) = value.as_array().and_then(|vec| vec.get(index)) else {
		return ValidationErrorWithPath::custom(format!("missing entry at index '{index}'"));
	};

	let mut errors = inspect(target);
	errors.prepend_array_index(index);
	errors
}

#[must_use]
pub(crate) fn expect_object_iterate<'a>(
	value: &'a serde_json::Value,
	mut inspect: impl FnMut((&'a String, &'a serde_json::Value)) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(object) = value.as_object() else {
		return ValidationErrorWithPath::custom("expected object");
	};

	let mut errors = Vec::with_capacity(0);
	for entry in object {
		let mut new_errors = inspect(entry);
		new_errors.prepend_object_index(entry.0);
		errors.extend(new_errors);
	}
	errors
}

#[must_use]
pub(crate) fn expect_array(value: &serde_json::Value) -> Vec<ValidationErrorWithPath> {
	if value.is_array() {
		return Vec::with_capacity(0);
	}
	ValidationErrorWithPath::custom("expected array")
}

#[must_use]
pub(crate) fn expect_array_iterate<'a>(
	value: &'a serde_json::Value,
	mut inspect: impl FnMut(&'a serde_json::Value) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(array) = value.as_array() else {
		return ValidationErrorWithPath::custom("expected array");
	};

	let mut errors = Vec::with_capacity(0);
	for (index, entry) in array.iter().enumerate() {
		let mut new_errors = inspect(entry);
		new_errors.prepend_array_index(index);
		errors.extend(new_errors);
	}
	errors
}

#[must_use]
pub(crate) fn expect_string<'a>(
	value: &'a serde_json::Value,
	inspect: impl FnOnce(&'a str) -> Vec<ValidationErrorWithPath>,
) -> Vec<ValidationErrorWithPath> {
	let Some(string) = value.as_str() else {
		return ValidationErrorWithPath::custom("expected string");
	};
	inspect(string)
}

#[must_use]
pub(crate) fn accept_any<T>(_value: T) -> Vec<ValidationErrorWithPath> {
	Vec::with_capacity(0)
}
