pub mod catalog;

#[derive(clap::Subcommand, Debug)]
pub enum Cmds {
    /// Docs go here.
    Catalog(catalog::Cmd),
}
