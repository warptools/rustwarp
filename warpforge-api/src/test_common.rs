use expect_test::Expect;
use serde::{Deserialize, Serialize};

#[inline]
pub(crate) fn json_roundtrip<'a, T: Deserialize<'a> + Serialize>(input: &'a str) -> String {
	let object: T = serde_json::from_str(input).expect("deserialization shouldn't fail");
	serde_json::to_string_pretty(&object).expect("serialization shouldn't fail")
}

#[inline]
pub(crate) fn assert_eq_json_roundtrip<'a, T: Deserialize<'a> + Serialize>(expect: &'a Expect) {
	let actual = json_roundtrip::<T>(expect.data());
	expect.assert_eq(&actual);
}

#[inline]
pub(crate) fn yaml_roundtrip<'a, T: Deserialize<'a> + Serialize>(input: &'a str) -> String {
	// serde_yml has `from_str` and `to_string` funcs, which are easier to use,
	// but they're incapable (?!) of doing the "singleton_map" thing on the root structure,
	// which... is a big problem since we literally always do that at our roots, lol.
	// So we do this more complex buf dance.

	let deserializer = serde_yml::Deserializer::from_str(input);
	let object: T = serde_yml::with::singleton_map_recursive::deserialize(deserializer)
		.expect("deserialization shouldn't fail");

	let mut buf = Vec::new();
	let mut serializer = serde_yml::Serializer::new(&mut buf);
	serde_yml::with::singleton_map_recursive::serialize(&object, &mut serializer)
		.expect("serialization shouldn't fail");
	return String::from_utf8(buf).expect("please, strings");
}

#[inline]
pub(crate) fn assert_eq_yaml_roundtrip<'a, T: Deserialize<'a> + Serialize>(expect: &'a Expect) {
	let actual = yaml_roundtrip::<T>(expect.data());
	expect.assert_eq(&actual);
}
