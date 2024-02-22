use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Lists installation candidates
    List,
    /// Uninstalls the candidate
    Uninstall { name: String, ver: Option<String> },
    /// Installs the [candidate] with optional [version]
    Install { name: String, ver: Option<String> },
    /// Lists items that are installed on this machine
    Installed,
}
