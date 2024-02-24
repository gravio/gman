use std::thread;
use std::time::Duration;
use std::{cmp::min, fmt::Write};

use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use tabled::grid::records::vec_records::CellInfo;

use std::{fs::File, io::BufReader, path::Path, process::Command};

use crate::candidate::{
    Candidate, InstallationCandidate, InstalledProduct, SearchCandidate, TablePrinter,
};
use crate::gman_error::MyError;
use crate::platform::Platform;
use crate::{get_build_id_by_candidate, get_builds, product, CandidateRepository, ClientConfig};

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

    fn get_valid_repositories_for_platform(&self) -> Vec<&CandidateRepository> {
        /* Platform to restrict our repos to */
        let platform: Option<Platform> = Platform::platform_for_current_platform();

        let valid_repositories: Vec<&CandidateRepository> = self
            .config
            .repositories
            .iter()
            .filter(|repo| {
                (repo.repository_folder.is_some() || repo.repository_server.is_some())
                    && (repo.platforms.is_empty()
                        || (platform.is_some()
                            && repo.platforms.contains(platform.as_ref().unwrap())))
            })
            .collect();

        if valid_repositories.is_empty() {
            log::warn!("No repositories available for searching. Either no repositories are known that match your current platform, or they dont have folder/server set");
        }

        valid_repositories
    }

    /// Lists the available candidates of Gravio items to install
    ///
    /// The list of candidates is retrieved from the repoository server defined in the [ClientConfig]
    pub async fn list_candidates(
        &self,
        name: Option<&str>,
        version: Option<&str>,
    ) -> Result<Vec<InstallationCandidate>, Box<dyn std::error::Error>> {
        log::debug!(
            "Listing candidates: name: {:#?}, version: {:#?}",
            name,
            version
        );

        log::debug!("{:#?}", self.config);

        let mut candidates: Vec<InstallationCandidate> = Vec::new();
        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

        let current_platform = Platform::platform_for_current_platform();
        if current_platform.is_none() {
            return Err(Box::new(MyError::new(
                "Cant get candidate builds for platform, current platform is not supported",
            )));
        }
        let current_platform = current_platform.unwrap();

        let valid_repositories = self.get_valid_repositories_for_platform();

        let products: Vec<&product::Product> = vec![
            &*product::PRODUCT_GRAVIO_HUBKIT,
            &*product::PRODUCT_GRAVIO_STUDIO,
        ];

        let mut builds = get_builds(
            &http_client,
            current_platform,
            &valid_repositories,
            &products,
        )
        .await?;

        candidates.append(&mut builds);

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
            if candidate.product_name.to_lowercase() == lower_name {
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
                println!("Successfully uninstalled {}", &candidate.product_name);
                Ok(())
            }
            None => {
                eprintln!("No item named {} found on system, cannot uninstall", &name);
                Err(Box::new(MyError::new("No item found")))
            }
        }
    }

    pub async fn install(
        &self,
        candidate: &SearchCandidate,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "Setting up installation prep for {} @ {}",
            &candidate.product_name,
            &candidate.version_or_identifier_string(),
        );

        /* Locate the resource (check if in cache, if not, check online) */
        let cache_path = self.locate_in_cache(candidate);
        if let Some(p) = cache_path {
            log::debug!(
                "Found installation executable for {}@{} in path",
                &candidate.product_name,
                &candidate.version_or_identifier_string()
            );
        } else {
            /* Download the resource (to cache) */
            log::debug!(
                "Installation executable for {}@{} not found in cache, attempting to download from repository",
                &candidate.product_name,
                &candidate.version_or_identifier_string()
            );

            let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

            let valid_repositories = self.get_valid_repositories_for_platform();

            let result =
                get_build_id_by_candidate(&http_client, candidate, &valid_repositories).await?;

            match result {
                Some(found) => {
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
                None => println!("No candidates found"),
            }
        }

        /* Launch installer */

        Ok(())
    }

    /// Attempts to locate the installer for the candiate in the locale cache
    fn locate_in_cache(&self, candidate: &SearchCandidate) -> Option<&Path> {
        None
    }
    /// Lists items installed to this machine
    pub fn get_installed(&self) -> Vec<InstalledProduct> {
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

    fn get_installed_windows(&self) -> Result<Vec<InstalledProduct>, Box<dyn std::error::Error>> {
        let mut installed: Vec<InstalledProduct> = Vec::new();
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
                    let package_full_name = vec[2].trim().to_owned();

                    let installed_product: InstalledProduct = InstalledProduct {
                        product_name: product::PRODUCT_GRAVIO_STUDIO.name.to_owned(),
                        version,
                        package_name: package_full_name,
                    };

                    installed.push(installed_product);
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

                    let installed_product = InstalledProduct {
                        product_name: product::PRODUCT_GRAVIO_HUBKIT.name.to_owned(),
                        version: version.to_owned(),
                        package_name: identifier.to_owned(),
                    };

                    installed.push(installed_product);
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
        {}

        Ok(installed)
    }

    /// Formats a list of Gravio Candidate items into a table and prints to stdout
    pub fn format_candidate_table<'a>(&self, candidates: Vec<impl Into<TablePrinter>>) {
        log::debug!(
            "Formatting candidate list with {} candidates",
            candidates.len()
        );

        // let lll = candidates[0];
        // let abc: TablePrinter = lll.into();
        let mut data = candidates
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<TablePrinter>>();

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
        builder.push_record(["Name", "Version", "Identifier", "Flavor"]);
        for item in &data {
            builder.push_record([&item.name, &item.version, &item.identifier, &item.flavor]);
        }
        if data.is_empty() {
            builder.push_record(["No candidates available"]);
        }

        let mut table = builder.build();

        table
            .with(Style::sharp())
            .with(Modify::new(Rows::first()).with(Alignment::center()));

        if data.is_empty() {
            table
                .modify((1, 0), tabled::settings::Span::column(3))
                .modify((1, 0), Alignment::center());
        }

        println!("{table}");
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Deserializer};
    use serde_json::Value;

    use crate::{
        candidate::{Candidate, SearchCandidate},
        cli::Target,
        download_artifact, get_build_id_by_candidate,
        product::{self, Product},
        Client, TeamCityArtifacts, TeamCityBranch, TeamCityBuild, TeamCityBuilds, TeamCityRoot,
    };

    #[tokio::test]
    async fn candidates() {
        simple_logger::SimpleLogger::new().env().init().unwrap();

        let c = Client::load().expect("Failed to load client");
        let candidates = c.list_candidates(None, None).await.unwrap();
        assert!(!candidates.is_empty());
        println!("lmao");
    }

    #[tokio::test]
    async fn get_build_id() {
        simple_logger::SimpleLogger::new().env().init().unwrap();

        let p = &product::PRODUCT_GRAVIO_HUBKIT;
        let candidate =
            SearchCandidate::new(p.name, Some("5.2.0-7015"), None, Some("WindowsHubkit")).unwrap();

        let c = Client::load().expect("Failed to load client");

        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

        let vv = c.get_valid_repositories_for_platform();

        match get_build_id_by_candidate(&http_client, &candidate, &vv).await {
            Ok(s) => match s {
                None => {
                    assert!(false, "Expected results, but got empty")
                }
                Some(ss) => {
                    assert!(!ss.remote_id.is_empty(), "expected a valid candidate with a remote id, got a candidate with nothing filled in")
                }
            },
            Err(_) => {
                assert!(false, "Expected a valid candidate with a remote id from build server, got no results instead");
            }
        }
    }

    #[tokio::test]
    async fn get_build_id_by_identifier_name() {
        simple_logger::SimpleLogger::new().env().init().unwrap();

        let p = &product::PRODUCT_GRAVIO_HUBKIT;
        let candidate = SearchCandidate::new(p.name, None, Some("develop"), None).unwrap();

        let c = Client::load().expect("Failed to load client");

        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

        let vv = c.get_valid_repositories_for_platform();

        match get_build_id_by_candidate(&http_client, &candidate, &vv).await {
            Ok(s) => match s {
                None => {
                    assert!(false, "Expected results, but got empty")
                }
                Some(ss) => {
                    assert!(!ss.remote_id.is_empty(), "expected a valid candidate with a remote id, got a candidate with nothing filled in")
                }
            },
            Err(_) => {
                assert!(false, "Expected a valid candidate with a remote id from build server, got no results instead");
            }
        }
    }

    #[tokio::test]
    async fn get_build_id_by_no_results() {
        simple_logger::SimpleLogger::new().env().init().unwrap();

        let p = &product::PRODUCT_GRAVIO_HUBKIT;
        let candidate = SearchCandidate::new(
            p.name,
            None,
            Some("1a361e15-27e2-48b1-bc8b-054d9ab8c435"),
            None,
        )
        .unwrap();

        let c = Client::load().expect("Failed to load client");

        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

        let vv = c.get_valid_repositories_for_platform();

        match get_build_id_by_candidate(&http_client, &candidate, &vv).await {
            Ok(s) => {
                assert!(
                    s.is_none(),
                    "Expected there to be no results, but found some"
                )
            }
            Err(_) => {
                assert!(false, "Expected no results, but got an error instead");
            }
        }
    }

    #[tokio::test]
    async fn install_hubkit_non_existant() {
        let c = Client::load().expect("Failed to load client");
        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();
        let vv = c.get_valid_repositories_for_platform();

        let target: Target = Target::Identifier("lmao".to_owned());

        let candidate = SearchCandidate::new(
            product::PRODUCT_GRAVIO_HUBKIT.name,
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(x) => Some(x.as_str()),
            },
            None,
        )
        .unwrap();

        c.install(&candidate).await.expect("Failed to install item");
    }

    #[test]
    fn deserde_artifacts() {
        let r = r#"{
            "count": 1
        }"#;

        let val = serde_json::from_str::<TeamCityArtifacts>(r);
        assert!(val.is_ok());
    }

    #[test]
    fn deserde_build() {
        let r = r#"{
            "id": 20211,
            "number": "5.2.1-7043",
            "finishDate": "20240221T085516+0000",
            "artifacts": {
                "count": 1
            }
        }"#;

        let val = serde_json::from_str::<TeamCityBuild>(r);
        assert!(val.is_ok());
    }

    #[test]
    fn deserde_builds() {
        let r = r#"{
            "count": 1,
            "build": [
                {
                    "id": 20211,
                    "number": "5.2.1-7043",
                    "finishDate": "20240221T085516+0000",
                    "artifacts": {
                        "count": 1
                    }
                }
            ]
        }"#;

        let val = serde_json::from_str::<TeamCityBuilds>(r);
        assert!(val.is_ok());
    }

    #[test]
    fn deserde_branch() {
        let r = r#"{
			"name": "master",
			"builds": {
				"count": 1,
				"build": [
					{
						"id": 20211,
						"number": "5.2.1-7043",
						"finishDate": "20240221T085516+0000",
						"artifacts": {
							"count": 1
						}
					}
				]
			}
		}"#;

        let val = serde_json::from_str::<TeamCityBranch>(r);
        println!("{:#?}", val);
        assert!(val.is_ok());
    }

    #[tokio::test]
    async fn download_develop_hubkit() {
        simple_logger::SimpleLogger::new().env().init().unwrap();

        let c = Client::load().expect("Failed to load client");
        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();
        let vv = c.get_valid_repositories_for_platform();

        let target: Target = Target::Identifier("develop".to_owned());

        let p = &product::PRODUCT_GRAVIO_HUBKIT;

        let c = SearchCandidate::new(p.name, None, Some("develop"), None).unwrap();

        let with_build_id = get_build_id_by_candidate(&http_client, &c, &vv)
            .await
            .expect("expected to get build id during test for develop hubkit install")
            .expect("Expected build id to exist");

        let res = download_artifact(&with_build_id)
            .await
            .expect("Expected downlod not to fail");

        assert!(false)
    }
}
