use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

// FUTURE: Could be represneted as an enum, discriminating on the first char being '/' or '$'
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromStr, Display)]
pub struct SandboxPort(pub String);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GatherDirective {
	pub from: SandboxPort,
	pub packtype: Option<crate::content::Packtype>,
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
	pub command: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub network: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionScript {
	pub interpreter: String,
	pub contents: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub network: Option<bool>,
}
