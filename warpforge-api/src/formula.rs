use std::ops::Not;

use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

// FUTURE: Could be represneted as an enum, discriminating on the first char being '/' or '$'
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromStr, Display)]
pub struct SandboxPort(String);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GatherDirective {
	from: SandboxPort,
	packtype: Option<crate::content::Packtype>,
	// TODO:
	// filters: Option<FilterMap>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Action {
	#[serde(rename = "echo")]
	Echo,
	#[serde(rename = "exec")]
	Execute(ActionExecute),
	#[serde(rename = "script")]
	Script(ActionScript),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionExecute {
	command: Vec<String>,
	#[serde(default, skip_serializing_if = "<&bool>::not")]
	network: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionScript {
	interpreter: String,
	contents: Vec<String>,
	#[serde(default, skip_serializing_if = "<&bool>::not")]
	network: bool,
}
