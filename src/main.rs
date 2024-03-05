mod app;
mod candidate;
mod cli;
mod client;
mod client_config;
mod gman_error;
mod platform;
mod product;
mod team_city;
mod util;
use candidate::{InstallationCandidate, Version};
use clap::Parser;
use cli::Commands;
use client_config::*;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

use crate::candidate::SearchCandidate;
use crate::cli::{Cli, Target};
use crate::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    if let Some(ll) = &cli.log_level {
        app::init_logging(Some(*ll));
    }

    let config = match ClientConfig::load_config(cli.config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load configuration file: {}", e);
            exit(1);
        }
    };

    match &cli.command {
        /* List */
        Some(Commands::Cache { clear, list: _ }) => {
            let client = Client::new(config);
            client.init();

            if *clear {
                match client.clear_cache() {
                    Ok(_) => {
                        println!("Cleared cache");
                        exit(0)
                    }
                    Err(e) => {
                        eprintln!("Failed to clear cache: {}", e);
                        exit(1);
                    }
                }
            } else {
                println!(
                    "Cache Directory: {}",
                    client.config.cache_directory.to_str().unwrap()
                );
                match client.list_cache() {
                    Some(items) => {
                        println!("Content Count: {}", items.len());
                        client.format_candidate_table(items, false, false);
                    }
                    None => {
                        println!("Nothing in cache");
                    }
                }
            }
            exit(0);
        }
        Some(Commands::List { show_installed }) => {
            let client = Client::new(config);
            client.init();

            let mut candidates = client
                .list_candidates(None, None)
                .await
                .expect("Failed to load candidates");
            let installed_candidates = client.get_installed();
            for installed in &installed_candidates {
                /* Keep Candidate in list if...
                 *   - not installed at all, or
                 *   - version is higher than installed
                 */
                if !show_installed {
                    candidates.retain_mut(|cd| !cd.product_equals(&installed))
                } else {
                    if !candidates
                        .iter()
                        .any(|x| x.product_equals(installed) && x.version == installed.version)
                    {
                        // TODO(nf): the flavor here is a dummy placeholder. This whole "show installed stuff even when not in the list" stuff is very messy
                        candidates.push(InstallationCandidate {
                            remote_id: String::default(),
                            repo_location: String::default(),
                            product_name: installed.product_name.to_owned(),
                            version: installed.version.clone(),
                            identifier: "--".to_owned(),
                            flavor: product::Flavor::empty(),
                            installed: true,
                        })
                    }
                }
            }
            /* set the Installed flag */
            for cd in candidates.iter_mut() {
                for installed in &installed_candidates {
                    if cd.product_equals(&installed) && cd.version == installed.version {
                        cd.installed = true;
                    }
                }
            }
            client.format_candidate_table(candidates, *show_installed, true);
            exit(0)
        }
        /* Uninstall */
        Some(Commands::Uninstall { name, ver }) => {
            let client = Client::new(config);
            client.init();

            let _ = client.uninstall(&name, ver.to_owned().map(|x| Version::new(&x)));
            exit(0)
        }
        /* Install */
        Some(Commands::Install {
            name,
            build_or_branch,
            flavor,
            automatic_upgrade,
        }) => {
            let client = Client::new(config);
            client.init();

            /* find product */
            let target: Target = match build_or_branch {
                Some(x) => Target::from_str(x.as_ref()).unwrap(),
                None => Target::Identifier("master".to_owned()),
            };

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
                flavor.as_ref().map(|x| x.as_str()),
                &client.config.products,
            );

            match candidate {
                Some(candidate) => {
                    println!(
                        "Installing {}@{}, flavor {}",
                        name,
                        target.to_string(),
                        candidate.flavor.id,
                    );
                    client
                        .install(&candidate, automatic_upgrade.to_owned())
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
            let client = Client::new(config);
            client.init();
            let candidates = client.get_installed();
            client.format_candidate_table(candidates, false, false);
            exit(0)
        }
        Some(Commands::Config { sample }) => {
            if *sample {
                let client = ClientConfig::make_sample();
                let name = app::CLIENT_CONFIG_FILE_NAME;
                let path = PathBuf::from_str("./")
                    .expect("Expected to make a valid path in current directory");

                let mut joined = path.join(name);
                let mut num: usize = 0;
                const MAX: usize = 200;
                while joined.exists() {
                    if num >= MAX {
                        eprintln!("Cannot create sample file, maximum number of tried exceeded (200). Try deleting files named {}", app::CLIENT_CONFIG_FILE_NAME);
                        exit(1);
                    }
                    num += 1;
                    let name = format!("{}.{}", name, num);
                    joined = path.join(name);
                }
                let stringified = serde_json::to_string_pretty(&client)
                    .expect("Expected to deserialize sample config json");
                std::fs::write(joined, stringified)?;
            }
        }

        None => {
            println!("use -h or --help to show help for this program");
        }
    }
    Ok(())
}
