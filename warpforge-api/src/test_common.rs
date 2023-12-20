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
