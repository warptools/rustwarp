use indexmap::IndexMap;
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

// The types here do a decent try at making invalid constructions unrepresentable,
// but some checks are validation logic rather than pure types.
// (As much as we try to use types to improve program quality, ultimately,
// types only help programmers -- and the real target audience here is the users, anyway.)
//
// There's three major regions where the types give up and the validation logic has to lift hard:
// - graph edge relationship checking
//   - for fairly clear reasons -- types can't do graphs
// - relationship binding position to value type matching
//   - partly because this would require the relations map to be some kind of pretty wild type theory construct like HLists, which... no.
//   - partly because relationship value source indirections (such as catalog references) make the type of the resolved value not immediately visible anyway.
// - relation binding site validity checks
//   - rules like "binding sites of the filesystem kind can't share a prefix of another mount UNLESS that one is writable" is far, far more complex than a type system can sensibly represent.
//
// So... yeah.  Some significant validation logic exists beyond the sheer types.
//
// After those fairly unavoidable challenges,
// we give up a little more ground because it's just pointless to try to hold it:
//
// - binding_affect has fields like "readwrite" which make sense for filesystems (that aren't mounts!) but not for vars and certainly not for happens-after's.
//

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ModuleCapsule {
	#[serde(rename = "module.v4")]
	V4(Module),
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Module {
	pub docs: Option<String>,
	pub usage: Option<String>,
	pub entrypoint: Option<ModuleEntrypoint>,
	pub steps: IndexMap<StepName, Step>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModuleEntrypoint {
	pub default: StepName,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, Display, FromStr)]
pub struct StepName(String);

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Step {
	// TODO: uh, oh boy.  This is...
	// what you want here is record-like but with flexible fields, which...
	// is barely known territory of type theory, afaik.  HLists get over there, but those are IME a very bad idea.
	// Uni-typing the value and having business logic rules for all the bizarre combos is possible and what's written here now, but disappointingly low on validation power.
	// Splitting this up into multiple maps and doing entirely custom deserialize logic might actually be the way.
	pub relations: IndexMap<RelationBinding, RelationValueSource>,

	// TODO name this better.
	pub binding_affect: Option<IndexMap<RelationBinding, BindingAffect>>,

	// TODO you know -- The Rest Of The Owl.
	// pub action: Action,
}

#[derive(
	Clone,
	Debug,
	Hash,
	PartialEq,
	Eq,
	DeserializeFromStr,
	SerializeDisplay,
	parse_display::Display,
	parse_display::FromStr,
)]
#[display("{}:{0}", style = "lowercase")]
pub enum RelationBinding {
	Fs(String),  // path
	Var(String), // variable name
	After(StepName),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindingAffect {
	// TODO placeholder
	// 'rw' marker goes here
	// possibly other things about tweaking posix bits that we now often ignore by default.
	// possibly even some well-known data validation rules.
}

// Note that the `binding_affect` field in Step is used to handle any "complex"
// feature such as specifying whether mount sites are read-write, etc.
// This is largely to avoid a desire to do a "kinded union" here, whether this data might be either map or string.
// (If that was easier, we'd probably have a "Complex" member of the RelationValueSource sum type, which decorates the other basic types.)
// Pulling those off a kinded-union semantic in serde is possible but tricky:
// https://serde.rs/string-or-struct.html is the serde core way, but doing your own whole deserializer, ouch.
// https://docs.rs/serde_with/latest/serde_with/struct.PickFirst.html might be a partial shortcut, but is still a remarkable amount of code.

#[derive(
	Clone,
	Debug,
	DeserializeFromStr,
	SerializeDisplay,
	parse_display::Display,
	parse_display::FromStr,
)]
#[display("{}:{0}", style = "lowercase")]
pub enum RelationValueSource {
	Literal(ValueLiteral),

	Catalog(CatalogRef),

	Mount(MountSpec),

	Ingest(IngestSpec),

	// TODO: the "after" bindings don't need a value and I don't know what to do with that.
}

#[derive(Clone, Debug, parse_display::Display, parse_display::FromStr)]
#[display("{}:{0}", style = "lowercase")]
pub enum ValueLiteral {
	Str(String),
	FsID(String),
}

#[derive(Clone, Debug, parse_display::Display, parse_display::FromStr)]
#[display("{module_name}:{release_name}:{item_name}")]
pub struct CatalogRef {
	pub module_name: ModuleName,
	pub release_name: ReleaseName,
	pub item_name: ItemName,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, parse_display::Display, parse_display::FromStr)]
pub struct ModuleName(#[from_str(regex = "[a-zA-Z0-9_/.-]+")] pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, parse_display::Display, parse_display::FromStr)]
pub struct ReleaseName(#[from_str(regex = "[a-zA-Z0-9_.-]+")] pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, parse_display::Display, parse_display::FromStr)]
pub struct ItemName(#[from_str(regex = "[a-zA-Z0-9_.-]+")] pub String);

#[derive(Clone, Debug, parse_display::Display, parse_display::FromStr)]
#[display("{mode}:{host_path}")]
pub struct MountSpec {
	pub mode: MountMode,
	pub host_path: String,
}

#[derive(Clone, Debug, parse_display::Display, parse_display::FromStr)]
pub enum MountMode {
	#[display("ro")]
	Readonly,
	#[display("rw")]
	Readwrite,
}

#[derive(Clone, Debug, parse_display::Display, parse_display::FromStr)]
#[display("{}:{0}", style = "lowercase")]
pub enum IngestSpec {
	Git(GitIngest),
}

#[derive(Clone, Debug, parse_display::Display, parse_display::FromStr)]
#[display("{host_path}:{git_ref}", style = "lowercase")]
pub struct GitIngest {
	host_path: String,
	git_ref: String,
	//subpath: Option<String>, // TODO: didn't expect optional to be hard here, but if display_parse supports this easily, I didn't find the incantation yet.
}
