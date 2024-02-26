use std::fs;
use std::str::FromStr as _;

use std::{fs::File, io::BufReader, process::Command};

use crate::candidate::{InstallationCandidate, InstalledProduct, SearchCandidate, TablePrinter};
use crate::gman_error::GravioError;
use crate::platform::Platform;
use crate::{
    app, download_artifact, get_build_id_by_candidate, get_builds, product, CandidateRepository,
    ClientConfig,
};

use tabled::settings::{object::Rows, Alignment, Modify, Style};

pub struct Client {
    pub config: ClientConfig,
}
impl Client {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let client_config = Client::load_config()?;
        app::init_logging();
        app::enable_logging(client_config.log_level);
        let c = Client::new(client_config);

        /* clear the temp directories */
        c.clear_temp();

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
        config.ensure_directories();
        Ok(config)
    }

    /// Deletes the temporary folder
    fn clear_temp(&self) {
        log::debug!("Clearing temporary folders");
        let app_temp_folder = std::env::temp_dir().join(app::APP_FOLDER_NAME);
        let _ = std::fs::remove_dir_all(app_temp_folder);
        let _ = std::fs::remove_dir_all(&self.config.temp_download_directory);
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
            return Err(Box::new(GravioError::new(
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
        version: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting to find uninstallation target for {}", &name);

        println!("Looking to uninstall an item: {}", name);
        let name_lower = name.to_lowercase();
        let installed = self.get_installed();
        let uninstall = installed.iter().find(|candidate| {
            if candidate.product_name.to_lowercase() == name_lower {
                if let Some(v) = &version {
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
                println!(
                    "Found uninstallation target. Attempting to uninstall {}",
                    candidate.product_name
                );
                candidate.uninstall()?;
                println!("Successfully uninstalled {}", &candidate.product_name);
                Ok(())
            }
            None => {
                eprintln!("No item named {} found on system, cannot uninstall", &name);
                Err(Box::new(GravioError::new("No item found")))
            }
        }
    }

    pub async fn install(
        &self,
        search: &SearchCandidate,
        automatic_upgrade: Option<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "Setting up installation prep for {} @ {}",
            &search.product_name,
            &search.version_or_identifier_string(),
        );

        /* first uninstall old version */
        let all_installed = &self.get_installed();
        let already_installed = all_installed
            .iter()
            .find(|x| x.product_name.to_lowercase() == search.product_name.to_lowercase());
        if let Some(already) = already_installed {
            eprintln!("Product already installed on machine. Uninstalling before continuing...");
            already.uninstall()?;
            eprintln!("Successfully Uninstalled product, continuing with new installation");
        }

        /* Locate the resource (check if in cache, if not, check online) */
        let cached_candidate = self.locate_in_cache(search);
        let actual_candidate = if let Some(p) = cached_candidate {
            log::debug!(
                "Found installation executable for {}@{} in path",
                &search.product_name,
                &search.version_or_identifier_string()
            );

            if let None = search.version {
                if automatic_upgrade.is_none() {
                    /* version unspecified, prompt user to optionally fetch latest from build server */
                    println!("A candidate for installation has been found in the local cache, but since the version was unspecified it may be oudated. Would you like to check the remote repositories for updated versions? [y/N]");
                    println!("{:#?}", &p);
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer)?;
                    if Self::is_console_confirm(&buffer) {
                        println!("Will search for more recent versions, and will use this cached item as fallback");
                        todo!()
                    } else {
                        println!("Will not search for more recent versions, will install this cached item");
                        todo!()
                    }
                } else if automatic_upgrade.is_some() {
                    match automatic_upgrade.unwrap() {
                        false => {
                            println!("A candidate for installation has been found in the local cache. Because version information wasnt specified, it may be outdated, but automatic upgrade was false. Will install local cache version.");
                            todo!();
                        }
                        true => {
                            println!("A candidate for installation has been found in the local cache. Automatic upgrade is true, will attempt to find later version on build server and will use this cached item as fallback");
                            todo!()
                        }
                    };
                }
            }
            p
        } else {
            /* Download the resource (to cache) */
            log::debug!(
                "Installation executable for {}@{} not found in cache, attempting to download from repository",
                &search.product_name,
                &search.version_or_identifier_string()
            );

            let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();

            let valid_repositories = self.get_valid_repositories_for_platform();

            let result =
                get_build_id_by_candidate(&http_client, search, &valid_repositories).await?;

            match result {
                Some(found) => {
                    let _ = download_artifact(
                        &http_client,
                        &found.0,
                        &found.1,
                        &self.config.temp_download_directory,
                        &self.config.cache_directory,
                        self.config.teamcity_download_chunk_size,
                    )
                    .await?;

                    found.0
                }
                None => {
                    println!("No candidates found");
                    return Ok(());
                }
            }
        };

        /* Launch installer */
        let binary_path = actual_candidate.make_output_for_candidate(&self.config.cache_directory);
        actual_candidate.install(&binary_path)
    }

    /// Attempts to locate the installer for the candiate in the local cache
    fn locate_in_cache(&self, search: &SearchCandidate) -> Option<InstallationCandidate> {
        let mut found_candidates: Vec<InstallationCandidate> = Vec::new();

        match fs::read_dir(&self.config.cache_directory) {
            Ok(list_dir) => {
                for entry_result in list_dir {
                    if let Ok(entry) = entry_result {
                        if let Ok(fname) = entry.file_name().into_string() {
                            if let Ok(ci) = InstallationCandidate::from_str(fname.as_str()) {
                                found_candidates.push(ci);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read cache directory: {}", e);
                return None;
            }
        };

        /* Sort the candidates, in preference of Flavor, Version, Identifier */
        found_candidates.sort_by(|a, b| {
            let cmp_flavor = a.flavor.name.cmp(&b.flavor.name);

            if cmp_flavor == std::cmp::Ordering::Equal {
                let cmp_version = a.version.cmp(&b.version);
                if cmp_version == std::cmp::Ordering::Equal {
                    a.identifier.cmp(&b.identifier)
                } else {
                    cmp_version
                }
            } else {
                cmp_flavor
            }
        });

        /* Drop non platform, non product items */
        found_candidates.retain(|x| {
            (x.flavor.platform == search.flavor.platform)
                && (x.product_name.to_lowercase() == search.product_name.to_lowercase())
        });

        for found in found_candidates.into_iter() {
            /* if version is specified, that overrides everything, grab first matching one */
            if let Some(v) = &search.version {
                if v.to_lowercase() == found.version.to_lowercase() {
                    log::info!("Found exact version match in cache");
                    return Some(found);
                }
            }
            if let Some(i) = &search.identifier {
                if i.to_lowercase() == found.identifier.to_lowercase() {
                    log::info!("Found matching identifier in cache");
                    return Some(found);
                }
            }

            if search.version.is_none() && search.identifier.is_none() {
                log::info!("Found matching inexact unspecified version/identifier in cache");
                return Some(found);
            }
        }

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
                return Err(Box::new(GravioError::new(
                    "Failed to get installations: Studio",
                )));
            }
        }

        /* get HubKit */
        {
            let command = r#"
            foreach($obj in Get-ChildItem "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall") {
                $dn = $obj.GetValue('DisplayName')
                if($dn -ne $null -and $dn.Contains('Gravio HubKit')) {
                  $key_name = ($obj | Select-Object Name | Split-Path -Leaf).replace('}}', '}')
                  $ver = $obj.GetValue('DisplayVersion')
                  Write-Host $dn@$ver@$key_name
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
                    let identifier = hubkit_splits[2].trim().to_owned();

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
                return Err(Box::new(GravioError::new(
                    "Failed to get installations: HubKit",
                )));
            }
        }

        /* get Gravio Sensor Map */
        {}

        Ok(installed)
    }

    /// Whether the given string is any kind of confirmation (yes, y, etc)
    fn is_console_confirm(val: &str) -> bool {
        let affirmative: Vec<&str> = vec!["y", "yes"];
        affirmative.iter().any(|v| *v == val.trim().to_lowercase())
    }

    /// Formats a list of Gravio Candidate items into a table and prints to stdout
    pub fn format_candidate_table<'a>(
        &self,
        candidates: Vec<impl Into<TablePrinter>>,
        show_installed: bool,
        show_flavor: bool,
    ) {
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
        let header_record = {
            let mut header: Vec<&str> = vec!["Name", "Version", "Identifier"];
            if show_flavor {
                header.push("Flavor");
            }
            if show_installed {
                header.push("Installed");
            }
            header
        };
        let header_record_count = header_record.len();
        builder.push_record(header_record);
        for item in &data {
            let record = {
                let mut r = vec![
                    item.name.to_owned(),
                    item.version.to_owned(),
                    item.identifier.to_owned(),
                ];
                if show_flavor {
                    r.push(item.flavor.to_owned());
                }
                if show_installed && item.installed {
                    r.push(item.installed.to_string());
                }
                r
            };
            builder.push_record(record);
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
                .modify((1, 0), tabled::settings::Span::column(header_record_count))
                .modify((1, 0), Alignment::center());
        }

        println!("{table}");
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        candidate::SearchCandidate,
        cli::Target,
        download_artifact, get_build_id_by_candidate,
        product::{self},
        Client, TeamCityArtifacts, TeamCityBranch, TeamCityBuild, TeamCityBuilds,
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
                    assert!(!ss.0.remote_id.is_empty(), "expected a valid candidate with a remote id, got a candidate with nothing filled in")
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
                    assert!(!ss.0.remote_id.is_empty(), "expected a valid candidate with a remote id, got a candidate with nothing filled in")
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

        c.install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_hubkit_develop() {
        let c = Client::load().expect("Failed to load client");
        let target: Target = Target::Identifier("develop".to_owned());

        let candidate = SearchCandidate::new(
            product::PRODUCT_GRAVIO_HUBKIT.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
        )
        .unwrap();

        c.install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_hubkit_specific_version() {
        let c = Client::load().expect("Failed to load client");
        let target: Target = Target::Version("5.2.1-7049".to_owned());

        let candidate = SearchCandidate::new(
            product::PRODUCT_GRAVIO_HUBKIT.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
        )
        .unwrap();

        c.install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_studio_specific_version() {
        let c = Client::load().expect("Failed to load client");
        let target: Target = Target::Version("5.2.4683".to_owned());

        let candidate = SearchCandidate::new(
            product::PRODUCT_GRAVIO_STUDIO.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
        )
        .unwrap();

        c.install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[test]
    fn uninstall_hubkit() {
        let c = Client::load().expect("Failed to load client");

        let _ = c.uninstall("hubkit", None);
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

        let client = Client::load().expect("Failed to load client");
        let http_client: reqwest::Client = reqwest::Client::builder().build().unwrap();
        let vv = client.get_valid_repositories_for_platform();
        let p = &product::PRODUCT_GRAVIO_HUBKIT;

        let c = SearchCandidate::new(p.name, None, Some("develop"), None).unwrap();

        let with_build_id = get_build_id_by_candidate(&http_client, &c, &vv)
            .await
            .expect("expected to get build id during test for develop hubkit install")
            .expect("Expected build id to exist");

        let _ = download_artifact(
            &http_client,
            &with_build_id.0,
            &with_build_id.1,
            &client.config.temp_download_directory,
            &client.config.cache_directory,
            client.config.teamcity_download_chunk_size,
        )
        .await
        .expect("Expected downlod not to fail");

        assert!(false)
    }

    #[test]
    fn try_expand() {
        let expanded_no_percent = shellexpand::tilde("%temp%");
        println!("{:#?}", expanded_no_percent);
    }
}
