// Quick note about things NOT present here:
//
// There are not yet types for the catalog root data.
// Ideally, these would be in some kind of IPLD Prolly Tree structure.
// In practice, we're currently based on files (which we assume, but do not check, are in git)
// for all of the catalog tree itself, and only start using content-addressing from a CatalogModule on down.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use catverters_derive;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use derive_more::{FromStr, Display};


#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CatalogModuleCapsule {
    #[serde(rename = "catalogmodule.v1")]
    V1(CatalogModule),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogModule {
    name: String, // TODO figure out how to wrap this better.  "newtype" pattern?
    releases: IndexMap<ReleaseName, String>, // TODO some type for CIDs?  Or should we just leave these as opaque strings at this level?  ... let's have a CID.  Strict seems right in this spot.
    metadata: IndexMap<String, String>, // Actually really is just strings :) // FUTURE: I yet don't know how to do "any" with serde in a codec-agnostic way, if we did want to.
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub struct CatalogRef {
    module_name: ModuleName,
    release_name: ReleaseName,
    item_name: ItemName,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(FromStr, Display)] // Unwrap the newtype.  We'll remove "From" if implementing stricter validation.
pub struct ModuleName(String); // Does not currently accomplish anything other than naming and documentation.  FUTURE: some validation rules would be nice -- see comments below about how, though.

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(FromStr, Display)] // Unwrap the newtype.  We'll remove "From" if implementing stricter validation.
pub struct ReleaseName(String); // Does not currently accomplish anything other than naming and documentation.  FUTURE: some validation rules would be nice -- see comments below about how, though.

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(FromStr, Display)] // Unwrap the newtype.  We'll remove "From" if implementing stricter validation.
pub struct ItemName(String); // Does not currently accomplish anything other than naming and documentation.  FUTURE: some validation rules would be nice -- see comments below about how, though.


/*
About validation:

- Ideally, Serde knows about validation... because that means if it fails, the error comes out with line info!  That's a big deal.
- But, becoming unable to handle data that doesn't validate is annoying in other scenarios.  So, I'd probably also want to be able to disable having serde do validation sometimes, to be able to passthrough values without guff.
- I like the ideas espoused in https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/ , particular to use the type system to remember if something is validated or not...
  - ... but ISTM you need some language-level support for the concept of refinement types in order to really make that viable on structures with any degree of nesting of types.  Otherwise you're writing a LOT of near-duplicate types; it's a taint effect -- it spreads to everything containing it.
    - And if that's possible in Rust, I don't know how, yet.

I flirt with the idea of adding non-serialized fields that track whether something _did_ pass validation.  That lets one have a nice middle ground... *but*, it doesn't solve the Serde-halts-with-line-info desire.

Overall if we come to "pick one", the most useful is probably to validate during serde and at all times.

Note that we're also rife with things that aren't appropriate to validate during parsing.
For example:

- connectedness of graphs when using pipe syntax in actions: that's a whole-document check, so there's no value (nor real possibility) of doing it with during the first pass and emitting info with line numbers.
- whether some step is using mount and ingest features or not -- we _could_ turn that into a validate thing, and it might be nice to do so, but it has the type-explosion problem again.  Unless we can figure out refinement types or way to make validation conditional.
*/

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
