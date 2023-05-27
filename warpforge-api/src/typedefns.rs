use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

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

// TODO this one requires the string destructuring special workover.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
