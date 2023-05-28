use indexmap::IndexMap;
use indoc::indoc;

#[test]
fn test_deserialize_fixture() {
    let mut fixtures: IndexMap<&str, &str> = IndexMap::new();
    fixtures.insert(
        "fixture-1",
        indoc! {r#"
            {
                "actions": {
                    "main": {}
                },
                "export": {}
            }
        "#},
    );
    fixtures.insert(
        "fixture-2",
        indoc! {r#"
            {
                "scene": {
                    "fs": {
                        "/wow": "ware:asdf:qwer"
                    }
                },
                "actions": {
                    "main": {}
                },
                "export": {}
            }
        "#},
    );

    for (_name, fixture) in fixtures {
        let value: warpforge_api::Workflow = serde_json::from_str(fixture).unwrap();
        let reserialized = serde_json::to_string(&value).unwrap();
        let foobar: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let normalized = serde_json::to_string(&foobar).unwrap();
        assert_eq!(reserialized, normalized);
    }
}
