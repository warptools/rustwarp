use catverters_derive;
use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub enum ContentRef {
	#[discriminant = "ware"]
	Ware(WareID),
	#[discriminant = "catalog"]
	CatalogRef(crate::catalog::CatalogRef),
	// TODO Ingest, Mount, etc, etc.
}

#[derive(
	Clone,
	Debug,
	SerializeDisplay,
	PartialEq,
	Eq,
	Hash,
	DeserializeFromStr,
	catverters_derive::Stringoid,
)]
pub struct WareID {
	pub packtype: Packtype,
	pub hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromStr, Display)]
pub struct Packtype(pub String);
