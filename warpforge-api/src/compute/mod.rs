use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate as wfapi;

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workflow {
	scene: Option<Scene>,
	actions: IndexMap<String, Action>,
	export: IndexMap<String, Export>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Scene {
	fs: IndexMap<String, wfapi::content::ContentRef>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Action {
	// TODO
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Export {
	// TODO
}
