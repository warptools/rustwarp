use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::fmt;

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workflow {
    scene: Option<Scene>,
    actions: IndexMap<String, Action>,
    export: IndexMap<String, Export>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Scene {
    fs: IndexMap<String, ContentRef>,
}

// TODO this requires the string prefixmatching special workover.
#[derive(Clone, Debug, Deserialize, Serialize)]
enum ContentRef {
    Ware(WareID),
    // TODO CatalogRef, Ingest, etc, etc.
}

impl fmt::Display for ContentRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentRef::Ware(val) => write!(f, "ware:{}", val),
            // _ => panic!() // ??
            //_ => Err(serde::de::Error::unknown_variant(variant, &["alpha", "beta"])),
        }
    }
}
/*
// The below is close, but doesn't handle errors correctly yet.
impl std::str::FromStr for ContentRef {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (prefix, rest) = s.split_once(':').ok_or(Err)?;
        match prefix {
            "ware" => Ok(ContentRef::ware(WareID::FromStr(rest))),
            //_ => Err(serde::de::Error::unknown_variant(variant, &["alpha", "beta"])),
        }
    }
}
*/

// Note there's also a WILDLY powerful macro called `serde_with::serde_conv!`:
// The below was an approximate that didn't quite shake out, but was interesting:
/*
serde_with::serde_conv!(
    ContentRefAsString,
    ContentRef,
    |packme: &ContentRef| -> String {
        match packme {
            ContentRef::Ware(val) => "ware:".to_owned() + &val.packtype + ":" + &val.hash, // FUTURE: would be nice to compose this better.
        }
    },
    |packed: String| -> Result<_, std::convert::Infallible> {
        Ok(ContentRef::Ware(WareID {
            packtype: "x".to_string(),
            hash: "y".to_string(),
        }))
    }
);*/
// You'd probably put that to work by tagging `#[serde_with::serde_as(as = "ContentRefAsString")]` somewhere.  But again, didn't quite figure it out.

// TODO this one requires the string destructuring special workover.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct WareID {
    packtype: String,
    hash: String,
}

impl fmt::Display for WareID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.packtype, self.hash)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Action {
    // TODO
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Export {
    // TODO
}
