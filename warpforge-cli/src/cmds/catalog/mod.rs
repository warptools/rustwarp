#[derive(clap::Subcommand, Debug)]
pub enum Cmd {
	/// read-item is a plumbing command.  It looks up a value by "{moduleName}:{releaseName}:{itemName}" tuple and returns the result as a plain string.
	/// It is designed to be used easily within shell scripts.
	///
	/// Optional flags to the command can cause additonal data to be reported with line-break delimiters, or cause the command to operate in JSON mode.
	ReadItem(ReadItemCmdArgs),
}

use std::str::FromStr; // i feared this might matter in an unhygenic way but it does not appear to.

#[derive(clap::Args, Debug)]
pub struct ReadItemCmdArgs {
	// #[clap(parse(try_from_str = warpforge_api::catalog::CatalogRef::try_from))] // So this doesn't exist anymore(?) and you get errors from the automapper attempt, which applies as a fallback, and which only uses From and not TryFrom.
	//#[arg(value_parser = clap::value_parser!(warpforge_api::catalog::CatalogRef))] // This appears to also explicitly look for From and not TryFrom?  But Why??  And I see FromStr being used here: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=99878a7c31bf79048c92a77c06096969 why the fuck does that not work for me here?
	// Okay, I think I see a hint.  Doing other totally broken stuff still produces a similar resoundingly insane error.  So that error might not be coming from that macro at all.  It's the default clap discovery, which is missing the mark in some way that's maybe even valid.
	//#[clap(parse(try_from_str = warpforge_api::catalog::CatalogRef::from_str))]
	#[arg(value_parser = warpforge_api::catalog::CatalogRef::from_str)]
	// This seems "close" but I would love to know why it wants the error to be `Box<dyn Error + Send + Sync + 'static>`.  That's a lot of constraints, and especially the lifetime one makes no sense to me.
	catalog_ref: warpforge_api::catalog::CatalogRef,
}
