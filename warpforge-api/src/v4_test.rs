use super::v4::*;

use crate::test_common::assert_eq_json_roundtrip;
use crate::test_common::assert_eq_yaml_roundtrip;
use expect_test::expect;

// problematic: you can get a very opaque "parse failed" in a chained parse_display::FromStr.
// And we use those _a lot_.

#[test]
fn test_roundtrip() {
      let expect = expect![[r#"
    {
      "module.v4": {
        "entrypoint": {
          "default": "main"
        },
        "steps": {
          "main": {
            "relations": {
              "after:greet": "literal:str:oof",
              "fs:/peek/": "mount:ro:.",
              "fs:/app/busybox/": "catalog:warpsys.org/busybox:v123:linux-amd64-static",
              "var:PATH": "literal:str:/bin"
            }
          },
          "greet": {
            "relations": {}
          }
        }
      }
    }"#]];
	assert_eq_json_roundtrip::<ModuleCapsule>(&expect);

	assert_eq_yaml_roundtrip::<ModuleCapsule>(&expect![[r#"
    module.v4:
      entrypoint:
        default: main
      steps:
        main:
          relations:
            after:greet: literal:str:oof
            fs:/peek/: mount:ro:.
            fs:/app/busybox/: catalog:warpsys.org/busybox:v123:linux-amd64-static
            var:PATH: literal:str:/bin
        greet:
          relations: {}
"#]]);
}
