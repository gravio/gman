use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

use clap::{Parser, Subcommand};

use crate::gman_error::GManError;

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
    List {
        #[clap(
            short,
            long,
            help = "if true, shows results that may already be installed on your computer"
        )]
        show_installed: bool,
    },
    /// Uninstalls the candidate
    Uninstall { name: String, ver: Option<String> },
    /// Installs the [candidate] with optional [version]
    Install {
        name: String,
        build_or_branch: Option<String>,
        #[clap(short, long, help = "Product flavor (e.g.,, Sideloading, Arm64 etc)")]
        flavor: Option<String>,
        #[clap(
            short,
            long,
            help = "Whether to find newer build versions, if `build` isnt specified. Leave empty to be prompted."
        )]
        automatic_upgrade: Option<bool>,
    },
    /// Clears the cache of all matching criteria, or all of it, if nothing specified
    Cache {
        #[clap(short, long, help = "Whether to clear the cache")]
        clear: bool,
        #[clap(short, long, help = "List which candidates are cached on disk")]
        list: bool,
    },
    /// Lists items that are installed on this machine
    Installed,

    /// Deals with the configuration
    Config {
        #[clap(short, long, help = "Generates a new sample configuration file")]
        sample: bool,
    },
}

#[derive(Subcommand, Clone)]

pub enum ConfigCommand {
    New,
}

#[derive(Debug, PartialEq)]
pub enum Target {
    Version(String),
    Identifier(String),
}

impl ToString for Target {
    fn to_string(&self) -> String {
        match self {
            Target::Version(s) => s.to_owned(),
            Target::Identifier(s) => s.to_owned(),
        }
    }
}

impl FromStr for Target {
    type Err = GManError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match VERSION_REGEX.find_iter(s).next() {
            Some(c) => {
                let matches_vesion = c.as_str().to_owned();
                Ok(Target::Version(matches_vesion))
            }
            None => Ok(Target::Identifier(s.to_owned())),
        }
    }
}

lazy_static! {
    static ref VERSION_REGEX: Regex =
        Regex::new(r"^((\d{1,}+)[.-]?)+$").expect("Failed to create Version 2 regex");
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Target;

    #[test]
    fn parse_target_identifier() {
        let ver = Some("develop");
        let target: Target = match ver {
            Some(x) => Target::from_str(x.as_ref()).unwrap(),
            None => Target::Identifier("master".to_owned()),
        };

        assert_eq!(target, Target::Identifier("develop".to_owned()))
    }

    #[test]
    fn parse_target_version() {
        let ver = Some("5.2.1-7322");
        let target: Target = match ver {
            Some(x) => Target::from_str(x.as_ref()).unwrap(),
            None => Target::Identifier("master".to_owned()),
        };

        assert_eq!(target, Target::Version("5.2.1-7322".to_owned()))
    }

    #[test]
    fn target_to_string() {
        let target = Target::Identifier("master".to_owned());

        assert_eq!(target.to_string(), "master")
    }
}
