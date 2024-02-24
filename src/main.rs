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

use candidate::{Candidate, InstallationCandidate};
use std::process::exit;
use std::str::FromStr;
use team_city::*;

use crate::cli::{Cli, Target};
use crate::client::Client;
use crate::platform::Platform;

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
                candidates.retain_mut(|cd| !cd.product_equals(&installed))
            }
            c.format_candidate_table(candidates);
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
        Some(Commands::Install { name, ver, flavor }) => {
            let c = Client::load().expect("Couldnt load client");

            /* find product */
            let platform: Option<Platform> = Platform::platform_for_current_platform();
            let product_opt = &product::Product::from_name_and_platform(name, platform);
            if let None = product_opt {
                eprintln!("Failed to install. No product known: {}", name);
                exit(1)
            }

            let target: Target = match ver {
                Some(x) => Target::from_str(x.as_ref()).unwrap(),
                None => Target::Identifier("master".to_owned()),
            };
            println!("Installing {:#?}@{}", name, target.to_string());

            let candidate = Candidate {
                remote_id: None,
                name: name.to_owned(),
                version: match &target {
                    Target::Identifier(_) => String::default(),
                    Target::Version(x) => x.to_owned(),
                },
                identifier: match &target {
                    Target::Identifier(x) => x.to_owned(),
                    Target::Version(_) => String::default(),
                },
                description: None,
                installed: false,
                product: product_opt.unwrap().to_owned(),
            };
            c.install(&candidate).await.expect("Failed to install item");

            exit(0)
        }
        Some(Commands::Installed) => {
            let c = Client::load().expect("Couldnt load client");
            let candidates = c.get_installed();
            c.format_candidate_table(candidates);
            exit(0)
        }
        None => {
            println!("Default subcommand");
        }
    }
    Ok(())
}
