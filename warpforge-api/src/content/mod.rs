use catverters_derive;
use serde_with::{DeserializeFromStr, SerializeDisplay};

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub enum ContentRef {
	#[discriminant = "ware"]
	Ware(WareID),
	#[discriminant = "catalog"]
	CatalogRef(crate::catalog::CatalogRef),
	// TODO Ingest, Mount, etc, etc.
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub struct WareID {
	packtype: String,
	hash: String,
}
