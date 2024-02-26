mod app;
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
use std::process::exit;
use std::str::FromStr;
use team_city::*;

use crate::candidate::SearchCandidate;
use crate::cli::{Cli, Target};
use crate::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    match &cli.command {
        /* List */
        Some(Commands::List { show_installed }) => {
            let c = Client::load().expect("Couldnt load client");
            let mut candidates = c
                .list_candidates(None, None)
                .await
                .expect("Failed to load candidates");
            let installed_candidates = c.get_installed();
            for installed in &installed_candidates {
                /* Keep Candidate in list if...
                 *   - not installed at all, or
                 *   - version is higher than installed
                 */
                if !show_installed {
                    candidates.retain_mut(|cd| !cd.product_equals(&installed))
                }
            }
            /* set the Installed flag */
            for cd in candidates.iter_mut() {
                for installed in &installed_candidates {
                    println!(
                        "Check {} against {}, version {} against {}",
                        cd.product_name,
                        installed.product_name,
                        cd.make_version_4_parts(),
                        installed.version,
                    );
                    if cd.product_equals(&installed)
                        && cd.make_version_4_parts() == installed.version
                    {
                        cd.installed = true;
                    }
                }
            }
            c.format_candidate_table(candidates, true);
            exit(0)
        }
        /* Uninstall */
        Some(Commands::Uninstall { name, ver }) => {
            let c = Client::load().expect("Couldnt load client");
            let _ = c.uninstall(&name, ver.to_owned());
            exit(0)
        }
        /* Install */
        Some(Commands::Install {
            name,
            build_or_branch,
            flavor_str,
            automatic_upgrade,
        }) => {
            let c = Client::load().expect("Couldnt load client");

            /* find product */
            let target: Target = match build_or_branch {
                Some(x) => Target::from_str(x.as_ref()).unwrap(),
                None => Target::Identifier("master".to_owned()),
            };

            println!("Installing {}@{}", name, target.to_string());

            let candidate = SearchCandidate::new(
                name,
                match &target {
                    Target::Identifier(_) => None,
                    Target::Version(x) => Some(x.as_str()),
                },
                match &target {
                    Target::Identifier(x) => Some(x.as_str()),
                    Target::Version(_) => None,
                },
                flavor_str.as_ref().map(|x| x.as_str()),
            );

            match candidate {
                Some(candidate) => {
                    c.install(&candidate, automatic_upgrade.to_owned())
                        .await
                        .expect("Failed to install item");
                    println!("Successfully Installed {}", candidate.product_name);
                    exit(0);
                }
                None => {
                    eprintln!("Could not construct a Search Candidate from the input parameters. Check that the product/flavor exist");
                    exit(1)
                }
            }
        }
        Some(Commands::Installed) => {
            let c = Client::load().expect("Couldnt load client");
            let candidates = c.get_installed();
            c.format_candidate_table(candidates, false);
            exit(0)
        }
        None => {
            println!("Default subcommand");
        }
    }
    Ok(())
}
