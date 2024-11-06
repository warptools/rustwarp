use std::{cell::RefCell, io::Read};

use indexmap::IndexMap;
use serde::de::{DeserializeSeed, Deserializer, Unexpected, Visitor};
use serde_json::{Deserializer as JsonDeserializer, Number};

pub fn from_str(input: &str) -> serde_json::Result<ValuePos> {
	from_slice(input.as_bytes())
}

pub fn from_slice(input: &[u8]) -> serde_json::Result<ValuePos> {
	let position = RefCell::new(Position {
		line: 1,
		column: 1,
		byte_offset: 0,
	});
	let read = LineColReader::new(&position, input);

	let mut json_deserializer = JsonDeserializer::from_reader(read);
	let start_pos = position.borrow().clone();
	let deserializer = ValuePosDeserializer {
		start_pos,
		cur_pos: &position,
	};
	let mut value = deserializer.deserialize(&mut json_deserializer)?;
	json_deserializer.end()?;
	clean_value_positions(input, &mut value);
	Ok(value)
}

/// Using our approach, the positions we find can contain leading and trailing
/// whitespaces, colons, and commas. This function advances all start positions
/// and recedes all end positons to remove any leading or trailing garbage.
fn clean_value_positions(src: &[u8], value: &mut ValuePos) {
	clean_position(src, &mut value.start, &mut value.end);

	match &mut value.value {
		Value::Primitive(_) => {}
		Value::Array(children) => {
			for child in children {
				clean_value_positions(src, child);
			}
		}
		Value::Object(children) => {
			for (_, child) in children {
				clean_position(src, &mut child.key_start, &mut child.key_end);
				clean_value_positions(src, &mut child.value);
			}
		}
	}
}

#[inline]
fn clean_position(src: &[u8], start: &mut Position, end: &mut Position) {
	while start.byte_offset < end.byte_offset {
		let byte = src[start.byte_offset];
		if byte.is_ascii_whitespace() || byte == b',' || byte == b':' {
			start.byte_offset += 1;
			if byte == b'\n' {
				start.line += 1;
				start.column = 1;
			} else {
				start.column += 1;
			}
		} else {
			break;
		}
	}

	let mut compute_end_column = false;
	while start.byte_offset < end.byte_offset {
		let byte = src[end.byte_offset - 1];
		if byte.is_ascii_whitespace() || byte == b',' || byte == b':' {
			end.byte_offset -= 1;
			if byte == b'\n' {
				end.line -= 1;
				compute_end_column = true;
			} else if !compute_end_column {
				end.column -= 1;
			}
		} else {
			break;
		}
	}

	if compute_end_column {
		if start.line == end.line && start.byte_offset <= end.byte_offset {
			end.column = start.column + (end.byte_offset - start.byte_offset);
		} else {
			let mut new_line = end.byte_offset - 1;
			while start.byte_offset <= new_line && src[new_line] != b'\n' {
				new_line -= 1;
			}
			if start.byte_offset <= new_line {
				// Found '\n'
				end.column = end.byte_offset - new_line;
			} else {
				debug_assert!(false, "start.line == end.line should be true");
				end.column = start.column;
			}
		}
	}
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct Position {
	line: usize,
	column: usize,
	byte_offset: usize,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct ValuePos {
	start: Position,
	end: Position,
	value: Value,
}

/// Represents any valid JSON value with positions.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value {
	/// Represents a JSON string, number, bool, or null value.
	Primitive(serde_json::Value),

	/// Represents a JSON array.
	///
	/// ```
	/// # use serde_json::json;
	/// #
	/// let v = json!(["an", "array"]);
	/// ```
	Array(Vec<ValuePos>),

	/// Represents a JSON object.
	///
	/// Backed by a IndexMap, which preserves entries in the order they are
	/// inserted into the map. In particular, this allows JSON data to be
	/// deserialized into a Value and serialized to a string while retaining
	/// the order of map keys in the input.
	///
	/// ```
	/// # use serde_json::json;
	/// #
	/// let v = json!({ "an": "object" });
	/// ```
	Object(IndexMap<String, MapEntry>),
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct MapEntry {
	value: ValuePos,

	key_start: Position,
	key_end: Position,
}

impl ValuePos {
	pub fn to_serde(self) -> serde_json::Value {
		match self.value {
			Value::Primitive(value) => value,
			Value::Array(vec) => {
				serde_json::Value::Array(vec.into_iter().map(ValuePos::to_serde).collect())
			}
			Value::Object(index_map) => serde_json::Value::Object(
				index_map
					.into_iter()
					.map(|(k, v)| (k, v.value.to_serde()))
					.collect(),
			),
		}
	}
}

impl core::hash::Hash for Value {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Value::Primitive(value) => value.hash(state),
			Value::Array(vec) => vec.hash(state),
			Value::Object(map) => {
				// From serde_json::map::Map with feature "preserving_order".
				let mut kv = Vec::from_iter(map);
				kv.sort_unstable_by(|a, b| a.0.cmp(b.0));
				kv.hash(state);
			}
		}
	}
}

struct ValuePosDeserializer<'de> {
	start_pos: Position,
	cur_pos: &'de RefCell<Position>,
}

impl<'de> DeserializeSeed<'de> for ValuePosDeserializer<'de> {
	type Value = ValuePos;

	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ValuePosVisitor<'de> {
			start_pos: Position,
			cur_pos: &'de RefCell<Position>,
		}

		impl ValuePosVisitor<'_> {
			fn value_pos<E>(self, value: Value) -> Result<ValuePos, E>
			where
				E: serde::de::Error,
			{
				Ok(ValuePos {
					start: self.start_pos,
					end: self.cur_pos.borrow().clone(),
					value,
				})
			}
		}

		impl<'de> Visitor<'de> for ValuePosVisitor<'de> {
			type Value = ValuePos;

			fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
				fmt.write_str("json value")
			}

			// serde_json "abuses" visit_unit to indicate null values.
			fn visit_unit<E>(self) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.value_pos(Value::Primitive(serde_json::Value::Null))
			}

			fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.value_pos(Value::Primitive(serde_json::Value::Bool(value)))
			}

			fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.value_pos(Value::Primitive(serde_json::Value::Number(value.into())))
			}

			fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.value_pos(Value::Primitive(serde_json::Value::Number(value.into())))
			}

			fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				let value = Number::from_f64(value).ok_or_else(|| {
					serde::de::Error::invalid_value(Unexpected::Float(value), &"a float value")
				})?;
				self.value_pos(Value::Primitive(serde_json::Value::Number(value)))
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.visit_string(value.to_string())
			}

			fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.value_pos(Value::Primitive(serde_json::Value::String(value)))
			}

			fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::SeqAccess<'de>,
			{
				let mut vec = match access.size_hint() {
					Some(size) => Vec::with_capacity(size),
					None => Vec::new(),
				};

				loop {
					let start_pos = self.cur_pos.borrow().clone();
					let Some(entry) = access.next_element_seed(ValuePosDeserializer {
						start_pos,
						cur_pos: self.cur_pos,
					})?
					else {
						break;
					};

					vec.push(entry);
				}

				self.value_pos(Value::Array(vec))
			}

			fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::MapAccess<'de>,
			{
				let mut map = match access.size_hint() {
					Some(size) => IndexMap::with_capacity(size),
					None => IndexMap::new(),
				};

				loop {
					let key_start = self.cur_pos.borrow().clone();
					let Some(key) = access.next_key()? else {
						break;
					};
					let key_end = self.cur_pos.borrow().clone();
					let value = access.next_value_seed(ValuePosDeserializer {
						start_pos: key_end.clone(),
						cur_pos: self.cur_pos,
					})?;
					map.insert(
						key,
						MapEntry {
							value,
							key_start,
							key_end,
						},
					);
				}

				self.value_pos(Value::Object(map))
			}
		}

		// let start_pos = self.cur_pos.borrow().clone();
		let start_pos = self.start_pos.clone();
		deserializer.deserialize_any(ValuePosVisitor {
			start_pos,
			cur_pos: self.cur_pos,
		})
	}
}

struct LineColReader<'a, T: Read> {
	position: &'a RefCell<Position>,
	inner: T,
}

impl<'a, R: Read> LineColReader<'a, R> {
	fn new(position: &'a RefCell<Position>, inner: R) -> Self {
		Self { position, inner }
	}
}

impl<T: Read> Read for LineColReader<'_, T> {
	fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
		// Read <= 1 byte at a time.
		if buffer.is_empty() {
			return Ok(0);
		}
		let len = self.inner.read(&mut buffer[..1])?;

		if len == 1 {
			let mut position = self.position.borrow_mut();
			if buffer[0] == b'\n' {
				position.line += 1;
				position.column = 1;
			} else {
				position.column += 1;
			}
			position.byte_offset += len;
		} else {
			debug_assert_eq!(len, 0);
		}
		Ok(len)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use expect_test::expect_file;
	use indoc::indoc;

	#[test]
	fn positions_from_str_simple() {
		let json = indoc! { r#"
			{
				"json": "some value"
			}
		"# };

		let parsed = from_str(json).unwrap();
		assert_eq!(parsed.start.line, 1);
		assert_eq!(parsed.end.line, 3);

		let expected = expect_file!["../tests/positions_from_str_simple.expected.txt"];
		expected.assert_debug_eq(&parsed);
	}

	#[test]
	fn positions_from_str() {
		// Strange leading commas are intentional and for testing!
		let json = indoc! { r#"
			{
				"string": "some value",
				"number": 5
				, "array": [1, "entry"],
				"bool": true,
				"null": null,
				"map": {
					"map-entry": false,
					"nested": [[[true]]]
				},
				"leading-commas": [
					"arr-entry"
					, {
						"a": "b",
						"c": "d"
					}
				]
			}
		"# };

		let parsed = from_str(json).unwrap();
		let expected = expect_file!["../tests/positions_from_str.expected.txt"];
		expected.assert_debug_eq(&parsed);
	}

	#[test]
	fn clean_positions_spaces() {
		let src = "     \"some string\"     ";
		let mut value = ValuePos {
			start: Position {
				line: 55,
				column: 0,
				byte_offset: 0,
			},
			end: Position {
				line: 55,
				column: src.len(),
				byte_offset: src.len(),
			},
			value: Value::Primitive(serde_json::Value::String(src.to_string())),
		};
		clean_value_positions(src.as_bytes(), &mut value);
		assert_eq!(value.start.byte_offset, 5);
		assert_eq!(value.start.column, 5);
		assert_eq!(value.start.line, 55);
		assert_eq!(value.end.byte_offset, src.trim_end().len());
		assert_eq!(value.end.column, src.trim_end().len());
		assert_eq!(value.end.line, 55);
	}

	#[test]
	fn clean_positions_variation() {
		let src = "  :\t  \"some string\",,   ";
		let column = 10;
		let mut value = ValuePos {
			start: Position {
				line: 55,
				column,
				byte_offset: 1,
			},
			end: Position {
				line: 55,
				column: column + src.len() - 1,
				byte_offset: src.len(),
			},
			value: Value::Primitive(serde_json::Value::String(src.to_string())),
		};
		clean_value_positions(src.as_bytes(), &mut value);
		assert_eq!(value.start.byte_offset, 6);
		assert_eq!(value.start.column, column + 5);
		assert_eq!(value.start.line, 55);
		assert_eq!(value.end.byte_offset, 19);
		assert_eq!(value.end.column, column + 18);
		assert_eq!(value.end.line, 55);
	}

	#[test]
	fn clean_positions_multiline_input() {
		let src = "\n\n :  \"some string\"  \n\n ,    \n";
		let mut value = ValuePos {
			start: Position {
				line: 55,
				column: 0,
				byte_offset: 0,
			},
			end: Position {
				line: 60,
				column: 0,
				byte_offset: src.len(),
			},
			value: Value::Primitive(serde_json::Value::String(src.to_string())),
		};
		clean_value_positions(src.as_bytes(), &mut value);
		assert_eq!(value.start.byte_offset, 6);
		assert_eq!(value.start.column, 5);
		assert_eq!(value.start.line, 57);
		assert_eq!(value.end.byte_offset, 19);
		assert_eq!(value.end.column, 18);
		assert_eq!(value.end.line, 57);
	}

	#[test]
	fn clean_positions_multiline_value() {
		let src = "\0\0:\n   \"some\nmultiline\nstring\"\n  ,\n    \0\n\0";
		let mut value = ValuePos {
			start: Position {
				line: 55,
				column: 10,
				byte_offset: 2,
			},
			end: Position {
				line: 60,
				column: 5,
				byte_offset: src.len() - 3,
			},
			value: Value::Primitive(serde_json::Value::String(src.to_string())),
		};
		clean_value_positions(src.as_bytes(), &mut value);
		assert_eq!(value.start.byte_offset, 7);
		assert_eq!(value.start.column, 4);
		assert_eq!(value.start.line, 56);
		assert_eq!(value.end.byte_offset, 30);
		assert_eq!(value.end.column, 8);
		assert_eq!(value.end.line, 58);
	}
}
