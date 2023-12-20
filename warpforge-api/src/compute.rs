use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate as wfapi;

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workflow {
	pub scene: Option<Scene>,
	pub actions: IndexMap<String, Action>,
	pub export: IndexMap<String, Export>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Scene {
	pub fs: IndexMap<String, wfapi::content::ContentRef>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Action {
	// TODO
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Export {
	// TODO
}
