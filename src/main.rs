mod candidate;
mod cli;
mod client;
mod client_config;
mod gman_error;
mod platform;
mod product;
mod team_city;
use clap::Parser;
use cli::Commands;
use client_config::*;
use simple_logger::SimpleLogger;

use indicatif::{ProgressBar, ProgressState, ProgressStyle};

use candidate::Candidate;
use std::process::exit;
use team_city::*;

use crate::cli::Cli;
use crate::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // This is where we will setup our HTTP client requests.
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let cli = Cli::parse();

    match &cli.command {
        /* List */
        Some(Commands::List) => {
            let c = Client::load().expect("Couldnt load client");
            let mut candidates = c
                .list_candidates(None, None)
                .await
                .expect("Failed to load candidates");
            let installed_candidates = c.get_installed();
            for installed in installed_candidates {
                candidates.retain_mut(|cd| cd != &installed)
            }
            c.format_candidate_table(&candidates);
            exit(0)
        }
        /* Uninstall */
        Some(Commands::Uninstall { name, ver }) => {
            let c = Client::load().expect("Couldnt load client");
            print!("uninstalling an item: {:?}", name);

            let _ = c.uninstall(&name, ver);
            exit(0)
        }
        /* Install */
        Some(Commands::Install { name, ver }) => {
            let c = Client::load().expect("Couldnt load client");

            let version = match ver {
                Some(x) => x.to_owned(),
                None => "master".to_owned(),
            };
            println!("Installing {:#?}@{:#?}", name, version);

            let candidate = Candidate {
                remote_id: None,
                name: name.to_owned(),
                version,
                identifier: "".to_owned(),
                description: None,
                installed: false,
                product: &product::Product {
                    name: "",
                    teamcity_id: "",
                },
            };
            c.install(&candidate).await.expect("Failed to install item");

            exit(0)
        }
        Some(Commands::Installed) => {
            let c = Client::load().expect("Couldnt load client");
            let candidates = c.get_installed();
            c.format_candidate_table(&candidates);
            exit(0)
        }
        None => {
            println!("Default subcommand");
        }
    }
    Ok(())
}
