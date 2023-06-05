// Quick note about things NOT present here:
//
// There are not yet types for the catalog root data.
// Ideally, these would be in some kind of IPLD Prolly Tree structure.
// In practice, we're currently based on files (which we assume, but do not check, are in git)
// for all of the catalog tree itself, and only start using content-addressing from a CatalogModule on down.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CatalogModuleCapsule {
    #[serde(rename = "catalogmodule.v1")]
    V1(CatalogModule),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogModule {
    name: String, // TODO figure out how to wrap this better.  "newtype" pattern?
    releases: IndexMap<String, String>, // TODO ReleaseName, some type for CIDs
    metadata: IndexMap<String, String>, // Actually really is just strings :) // FUTURE: I yet don't know how to do "any" with serde in a codec-agnostic way, if we did want to.
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use indoc::indoc;

    #[test]
    fn roundtripping_catalogmodulecapsule() {
        let mut fixtures: IndexMap<&str, &str> = IndexMap::new();
        fixtures.insert(
            "fixture-1",
            indoc! {r#"
		{
			"catalogmodule.v1": {
				"name": "warpsys.org/gawk",
				"releases": {
					"v5.1.1": "zM5K3TQtn57apb6hjS6A2LHsDW6FnD3m4xtECuZMqYLNMP42FxVsHxFbFEJ5jUrupoxi2Uv"
				},
				"metadata": {}
			}
		}
        "#},
        );

        for (_name, fixture) in fixtures {
            let value: super::CatalogModuleCapsule = serde_json::from_str(fixture).unwrap();
            let reserialized = serde_json::to_string(&value).unwrap();
            let foobar: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let normalized = serde_json::to_string(&foobar).unwrap();
            assert_eq!(reserialized, normalized);
        }
    }
}
