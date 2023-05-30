use catverters_derive;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use serde_with::{SerializeDisplay, DeserializeFromStr};

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

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
enum ContentRef {
    #[discriminant = "ware"]
    Ware(WareID),
    // TODO CatalogRef, Ingest, etc, etc.
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
struct WareID {
    packtype: String,
    hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Action {
    // TODO
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Export {
    // TODO
}
