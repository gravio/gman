use std::thread;
use std::time::Duration;
use std::{cmp::min, fmt::Write};

use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use tabled::grid::records::vec_records::CellInfo;

use std::{fs::File, io::BufReader, path::Path, process::Command};

use crate::candidate::Candidate;
use crate::gman_error::MyError;
use crate::platform::Platform;
use crate::{get_hubkit_builds, get_studio_builds, product, CandidateRepository, ClientConfig};

use tabled::{
    settings::{object::Rows, Alignment, Modify, Style},
    Table, Tabled,
};

pub struct Client {
    pub config: ClientConfig,
}
impl Client {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let client_config = Client::load_config()?;
        let c = Client::new(client_config);
        Ok(c)
    }
    pub fn new(config: ClientConfig) -> Self {
        log::debug!("Instantiating new gman client");
        Self { config }
    }

    /// Loads the config file, if any, from the 'gman.config' next to the gman executable
    pub fn load_config() -> Result<ClientConfig, Box<dyn std::error::Error>> {
        log::debug!("Loading gman client configuration");
        let file = File::open("./gman_config_client.json")?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let config: ClientConfig = serde_json::from_reader(reader)?;
        Ok(config)
    }

    /// Lists the available candidates of Gravio items to install
    ///
    /// The list of candidates is retrieved from the repoository server defined in the [ClientConfig]
    pub async fn list_candidates(
        &self,
        name: Option<&str>,
        version: Option<&str>,
    ) -> Result<Vec<Candidate>, Box<dyn std::error::Error>> {
        log::debug!(
            "Listing candidates: name: {:#?}, version: {:#?}",
            name,
            version
        );

        log::debug!("{:#?}", self.config);

        /* Platform to restrict our repos to */
        let platform: Option<&Platform> = if cfg!(windows) {
            Some(&Platform::Windows)
        } else {
            None
        };

        let valid_repositories: Vec<&CandidateRepository> = self
            .config
            .repositories
            .iter()
            .filter(|repo| {
                (repo.repository_folder.is_some() || repo.repository_server.is_some())
                    && (repo.platforms.is_empty()
                        || (platform.is_some() && repo.platforms.contains(platform.unwrap())))
            })
            .collect();

        if valid_repositories.is_empty() {
            log::warn!("No repositories available for searching. Either no repositories are known that match your current platform, or they dont have folder/server set");
            return Ok(Vec::new());
        }

        let mut candidates: Vec<Candidate> = Vec::new();
        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

        /* get Hubkits */
        let mut xyz = get_hubkit_builds(&http_client, &valid_repositories).await?;
        candidates.append(&mut xyz);

        /* get Studio */
        let mut xyz = get_studio_builds(&http_client, &valid_repositories).await?;
        candidates.append(&mut xyz);

        /* fake candidates */
        Ok(candidates)
    }

    pub fn uninstall(
        &self,
        name: &str,
        version: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting to find uninstallation target for {}", &name);

        let lower_name = name.to_lowercase();
        let installed = self.get_installed();
        let uninstall = installed.iter().find(|candidate| {
            if candidate.name.to_lowercase() == lower_name {
                if let Some(v) = version {
                    &candidate.version == v
                } else {
                    true
                }
            } else {
                false
            }
        });

        match uninstall {
            Some(candidate) => {
                log::debug!("Found uninstallation target, will attempt an uninstall");
                candidate.uninstall()?;
                println!("Successfully uninstalled {}", candidate.product.name);
                Ok(())
            }
            None => {
                eprintln!("No item named {} found on system, cannot uninstall", &name);
                Err(Box::new(MyError::new("No item found")))
            }
        }
    }

    pub async fn install<'a>(
        &self,
        candidate: &Candidate<'a>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "Setting up installation prep for {} @ {}",
            &candidate.name,
            &candidate.version
        );

        /* Locate the resource (check if in cache, if not, check online) */
        let cache_path = &self.locate_in_cache(candidate);
        if let Some(p) = cache_path {
            log::debug!(
                "Found installation executable for {}@{} in path",
                &candidate.name,
                &candidate.version
            );
        } else {
            /* Download the resource (to cache) */
            log::debug!(
                "Installation executable for {}@{} not found, attempting to download from repository",
                &candidate.name,
                &candidate.version
            );

            let mut downloaded = 0;
            let total_size = 231231231;

            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

            while downloaded < total_size {
                let new = min(downloaded + 223211, total_size);
                downloaded = new;
                pb.set_position(new);
                thread::sleep(Duration::from_millis(12));
            }

            pb.finish_with_message("downloaded");
        }

        /* Launch installer */

        Ok(())
    }

    /// Attempts to locate the installer for the candiate in the locale cache
    fn locate_in_cache(&self, candidate: &Candidate) -> Option<&Path> {
        None
    }
    /// Lists items installed to this machine
    pub fn get_installed(&self) -> Vec<Candidate> {
        log::debug!("Getting installed Gravio items");
        #[cfg(target_os = "windows")]
        {
            let candidates = self
                .get_installed_windows()
                .expect("Failed to get installed gravio items");
            candidates
        }
        #[cfg(target_os = "macos")]
        {}
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {}
    }

    fn get_installed_windows(&self) -> Result<Vec<Candidate>, Box<dyn std::error::Error>> {
        let mut installed: Vec<Candidate> = Vec::new();
        /* get Gravio Studio */
        {
            let command = r#"Get-AppxPackage | Where-Object {$_.Name -match ".*GravioStudio.*" } | Select Name, Version, PackageFullName"#;
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(command)
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                // Convert the output bytes to a string
                let result = String::from_utf8_lossy(&output.stdout);

                let studio_splits = result.split("\r\n").skip(3).next().map(|s| s.split(" "));
                if let Some(text) = studio_splits {
                    let vec: Vec<&str> = text.collect();
                    let version = vec[1].trim().to_owned();
                    let location = vec[2].trim().to_owned();

                    let p = &product::PRODUCT_GRAVIO_STUDIO_WINDOWS;

                    let c = Candidate {
                        remote_id: None,
                        name: p.name.to_owned(),
                        version,
                        identifier: location,
                        description: None,
                        installed: true,
                        product: p,
                    };
                    installed.push(c);
                }
            } else {
                // Print the error message if the command failed
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(MyError::new(
                    "Failed to get installations: Studio",
                )));
            }

            // Remove-AppxPackage
        }

        /* get HubKit */
        {
            // Uninstall-Package "Gravio HubKit  5.2.1.7032"
            let command = r#"
            foreach($obj in Get-ChildItem "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall") {
                $dn = $obj.GetValue('DisplayName')
                if($dn -ne $null -and $dn.Contains('Gravio HubKit')) {
                  $ver = $obj.GetValue('DisplayVersion')
                  Write-Host $dn@$ver@$obj
                }
              }"#;

            let output = Command::new("powershell")
                .arg("-Command")
                .arg(command)
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                // Convert the output bytes to a string
                let result = String::from_utf8_lossy(&output.stdout);
                if result.len() > 0 {
                    let hubkit_splits: Vec<&str> = result.split("@").collect();
                    let version = hubkit_splits[1].trim().to_owned();
                    let identifier = hubkit_splits[0].trim().to_owned();
                    let key_name = {
                        let key = hubkit_splits[2].trim();
                        let id_start_idx = key
                            .find('{')
                            .expect("Expected registry key to have open brace ({)");
                        let id_end_idx = key
                            .find('}')
                            .expect("Expected registry key to have close brace (})");
                        if id_start_idx + 1 >= key.len() || id_start_idx >= id_end_idx {
                            return Err(Box::new(MyError::new("Registry key was invalid, the opening brace was at the end of the string")));
                        }
                        &key[id_start_idx..id_end_idx + 1]
                    };

                    let p = &product::PRODUCT_GRAVIO_HUBKIT;
                    let c = Candidate {
                        remote_id: None,
                        name: p.name.to_owned(),
                        version,
                        identifier: key_name.to_owned(),
                        description: None,
                        installed: true,
                        product: p,
                    };
                    installed.push(c);
                }
            } else {
                // Print the error message if the command failed
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(MyError::new(
                    "Failed to get installations: HubKit",
                )));
            }
        }

        /* get Gravio Sensor Map */
        {
            // Uninstall-Package "Gravio HubKit  5.2.1.7032"
            // let command = r#"
            // foreach($obj in Get-ChildItem "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall") {
            //     $dn = $obj.GetValue('DisplayName')
            //     if($dn -ne $null -and $dn.Contains('Gravio Sensor Map')) {
            //       $ver = $obj.GetValue('DisplayVersion')
            //       Write-Host $dn@$ver
            //     }
            //   }"#;

            // let output = Command::new("powershell")
            //     .arg("-Command")
            //     .arg(command)
            //     .output()?;

            // // Check if the command was successful
            // if output.status.success() {
            //     // Convert the output bytes to a string
            //     let result = String::from_utf8_lossy(&output.stdout);
            //     if result.len() > 0 {
            //         let hubkit_splits: Vec<&str> = result.split("@").collect();
            //         let name = "Sensor Map".to_owned();
            //         let version = hubkit_splits[1].trim().to_owned();
            //         let identifier = hubkit_splits[0].trim().to_owned();

            //         let p = product::PRODUCT_

            //         let c = Candidate {
            //             remote_id: "".to_owned(),
            //             name,
            //             version,
            //             identifier,
            //             description: None,
            //             installed: true,
            //         };
            //         installed.push(c);
            //     }
            // } else {
            //     // Print the error message if the command failed
            //     eprintln!("PowerShell command failed:\n{:?}", output.status);
            //     return Err(Box::new(MyError::new("Failed to get installations: GSM")));
            // }
        }

        Ok(installed)
    }

    /// Formats a list of Gravio Candidate items into a table and prints to stdout
    pub fn format_candidate_table(&self, candidates: &Vec<Candidate>) {
        log::debug!(
            "Formatting candidate list with {} candidates",
            candidates.len()
        );

        let mut data: Vec<&Candidate<'_>> = candidates.iter().map(|x| x).collect();

        data.sort_by(|a, b| {
            let cmp_name = a.name.cmp(&b.name);

            if cmp_name == std::cmp::Ordering::Equal {
                b.version.cmp(&a.version)
            } else {
                cmp_name
            }
        });

        let mut builder = tabled::builder::Builder::default();
        // builder.push_column(["Name", "Version", "Lmao"]);
        // builder.push_column("Version");
        builder.push_record(["Name", "Version", "Identifier"]);
        for item in &data {
            builder.push_record([&item.name, &item.version, &item.identifier]);
        }
        if candidates.is_empty() {
            builder.push_record(["No candidates available"]);
        }

        let mut table = builder.build();

        table
            .with(Style::sharp())
            .with(Modify::new(Rows::first()).with(Alignment::center()));

        if candidates.is_empty() {
            table
                .modify((1, 0), tabled::settings::Span::column(3))
                .modify((1, 0), Alignment::center());
        }

        println!("{table}");
    }
}

#[cfg(test)]
mod tests {
    use crate::{candidate::Candidate, get_build_id_by_candidate, product, Client, TeamCityRoot};
    #[test]
    fn parse_xml() {
        let r = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <branches>
            <branch name="master">
                <builds count="1">
                    <build id="20200" number="5.2.1-7037">
                        <finishDate>20240219T065057+0000</finishDate>
                        <artifacts count="1"/>
                    </build>
                </builds>
            </branch>
            <branch name="GoogleDrive">
                <builds count="1">
                    <build id="20204" number="5.2.1-7039">
                        <finishDate>20240220T083702+0000</finishDate>
                        <artifacts count="1"/>
                    </build>
                </builds>
            </branch>
            <branch name="box">
                <builds count="1">
                    <build id="20199" number="5.2.1-7036">
                        <finishDate>20240216T134821+0000</finishDate>
                        <artifacts count="1"/>
                    </build>
                </builds>
            </branch>
            <branch name="develop">
                <builds count="1">
                    <build id="20192" number="5.2.1-7032">
                        <finishDate>20240215T022206+0000</finishDate>
                        <artifacts count="1"/>
                    </build>
                </builds>
            </branch>
            <branch name="experimental_endpoint">
                <builds count="1">
                    <build id="20205" number="5.2.1-7040">
                        <finishDate>20240220T084608+0000</finishDate>
                        <artifacts count="1"/>
                    </build>
                </builds>
            </branch>
        </branches>
        "#;

        let result: Result<TeamCityRoot, _> = serde_xml_rs::from_str(r);
        assert!(result.is_ok())
    }

    #[tokio::test]
    async fn candidates() {
        let c = Client::load().expect("Failed to load client");
        let candidates = c.list_candidates(None, None).await.unwrap();
        assert!(!candidates.is_empty());
        println!("lmao");
    }

    #[tokio::test]
    async fn get_build_id() {
        let p = &product::PRODUCT_GRAVIO_HUBKIT;
        let candidate = Candidate {
            description: None,
            identifier: "".to_owned(),
            name: p.name.to_owned(),
            remote_id: None,
            version: "5.2.0-7013".to_owned(),
            installed: false,
            product: p,
        };

        let c = Client::load().expect("Failed to load client");

        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

        let id = get_build_id_by_candidate(&http_client, &candidate, &c.config.repositories)
            .await
            .expect("Expected an internal Id");

        print!("{}", id);
    }
}
