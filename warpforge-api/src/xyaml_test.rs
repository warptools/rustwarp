use super::xyaml;
use expect_test::expect;
use serde::{Deserialize, Serialize};

#[test]
fn test_hello() {
	#[derive(Serialize, Deserialize)]
	struct CustomStruct {
		field1: String,
		field2: i32,
	}

	let object: CustomStruct = xyaml::deserialize_str(
		&r#"
field1: wow
field2: 15
"#,
	)
	.expect("deserialization shouldn't fail");

	expect![[r#"
    field1: 'wow'
    field2: 15
"#]]
	.assert_eq(
		xyaml::to_string(&object)
			.expect("serialization shouldn't fail")
			.as_str(),
	)
}
