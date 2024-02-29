use std::str::FromStr as _;
use std::{env, fs};

use std::{fs::File, io::BufReader, process::Command};

#[cfg(target_os = "windows")]
use crate::candidate::InstalledAppXProduct;
use crate::candidate::{
    InstallationCandidate, InstalledProduct, SearchCandidate, TablePrinter, Version,
};

use crate::gman_error::GManError;
use crate::platform::Platform;
use crate::{app, product, team_city, util, CandidateRepository, ClientConfig};

use tabled::settings::{object::Rows, Alignment, Modify, Style};

pub struct Client {
    pub config: ClientConfig,
    http_client: reqwest::Client,
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
        Self {
            config,
            http_client: reqwest::Client::builder().build().unwrap(),
        }
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

        let current_platform = Platform::platform_for_current_platform();
        if current_platform.is_none() {
            return Err(Box::new(GManError::new(
                "Cant get candidate builds for platform, current platform is not supported",
            )));
        }
        let current_platform = current_platform.unwrap();

        let valid_repositories = self.get_valid_repositories_for_platform();

        let products: Vec<&product::Product> = vec![
            &*product::PRODUCT_GRAVIO_HUBKIT,
            &*product::PRODUCT_GRAVIO_STUDIO,
            &*product::PRODUCT_HANDBOOK_X,
        ];

        let mut builds = team_city::get_builds(
            &self.http_client,
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
        version: Option<Version>,
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
                Err(Box::new(GManError::new("No item found")))
            }
        }
    }

    async fn download(
        &self,
        search: &SearchCandidate,
    ) -> Result<Option<InstallationCandidate>, Box<dyn std::error::Error>> {
        let valid_repositories = self.get_valid_repositories_for_platform();
        let result = team_city::get_with_build_id_by_candidate(
            &self.http_client,
            search,
            &valid_repositories,
        )
        .await?;

        match result {
            Some(found) => {
                let _ = team_city::download_artifact(
                    &self.http_client,
                    &found.0,
                    &found.1,
                    &self.config.temp_download_directory,
                    &self.config.cache_directory,
                    self.config.teamcity_download_chunk_size,
                )
                .await?;

                Ok(Some(found.0))
            }
            None => {
                println!("No candidates found");
                return Ok(None);
            }
        }
    }

    async fn get_build_server_version_if_higher_or_also_from_cache(
        &self,
        cached: InstallationCandidate,
        search: &SearchCandidate,
        valid_repositories: &Vec<&CandidateRepository>,
    ) -> Result<InstallationCandidate, Box<dyn std::error::Error>> {
        match team_city::get_with_build_id_by_candidate(
            &self.http_client,
            search,
            &valid_repositories,
        )
        .await
        {
            Ok(res) => match res {
                Some(found_on_server) => {
                    let sc = SearchCandidate {
                        version: Some((&found_on_server.0.version).clone()),
                        flavor: search.flavor.clone(),
                        identifier: Some(found_on_server.0.identifier.clone()),
                        product_name: search.product_name.clone(),
                    };
                    if let Some(new_found) = self.locate_in_cache(&sc) {
                        println!("Found most recent serer build id version in cache ({}), will skip download and returning", found_on_server.0.version);
                        return Ok(new_found);
                    }
                    if found_on_server.0.version > cached.version {
                        println!("Found a version on the server for this identifier that is greater than the one in cache (cached: {}, found: {}), will download and install from remote", cached.version, found_on_server.0.version);
                        let found_opt = self.download(search).await?;
                        match found_opt {
                            Some(with_id) => Ok(with_id),
                            None => {
                                eprintln!("Fetch request found an id on the build server but download request didn't find anything. This situation cannot be resolved by gman.");
                                return Err(Box::new(GManError::new(
                                    "Head fetch found id, but download found no id",
                                )));
                            }
                        }
                    } else {
                        println!("Cache is up to date with version ({}) on server, will skip downloading and install from cache", found_on_server.0.version);
                        Ok(cached)
                    }
                }
                None => {
                    log::info!("Repo returned correctly, but build id was not found on server. Will install from cache.");
                    Ok(cached)
                }
            },
            Err(e) => {
                log::error!("Encountered an error when contacting repository for up to date information. Installing from cache: {}", e);
                eprintln!("Encountered an error when contacting repository for up to date information. Will install the cached version");
                Ok(cached)
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

        /* Locate the resource (check if in cache, if not, check online) */
        let cached_candidate = self.locate_in_cache(search);

        let actual_candidate = match cached_candidate {
            Some(cached) => {
                log::debug!(
                    "Found installation executable for {}@{} in path",
                    &search.product_name,
                    &search.version_or_identifier_string()
                );

                if let None = search.version {
                    let valid_repositories = self.get_valid_repositories_for_platform();

                    match automatic_upgrade {
                        Some(should_upgrade) => match should_upgrade {
                            false => {
                                println!("A candidate for installation has been found in the local cache. Because version information wasnt specified, it may be outdated, but automatic upgrade was false. Will install local cache version.");
                                cached
                            }
                            true => {
                                println!("A candidate for installation has been found in the local cache. Automatic upgrade is true, will attempt to find later version on build server and will use this cached item as fallback");

                                self.get_build_server_version_if_higher_or_also_from_cache(
                                    cached,
                                    search,
                                    &valid_repositories,
                                )
                                .await?
                            }
                        },
                        None => {
                            /* version unspecified, prompt user to optionally fetch latest from build server */
                            println!("A candidate for installation has been found in the local cache, but since the version was unspecified it may be oudated. Would you like to check the remote repositories for updated versions? [y/N]");
                            println!("{:#?}", &cached);
                            let mut buffer = String::new();
                            std::io::stdin().read_line(&mut buffer)?;
                            if Self::is_console_confirm(&buffer) {
                                println!("Will search for more recent versions, and will use this cached item as fallback");
                                self.get_build_server_version_if_higher_or_also_from_cache(
                                    cached,
                                    search,
                                    &valid_repositories,
                                )
                                .await?
                            } else {
                                println!("Will not search for more recent versions, will install this cached item");
                                cached
                            }
                        }
                    }
                } else {
                    cached
                }
            }
            None => {
                /* Download the resource (to cache) */
                log::debug!(
                "Installation executable for {}@{} not found in cache, attempting to download from repository",
                &search.product_name,
                &search.version_or_identifier_string()
            );

                match self.download(search).await? {
                    Some(found) => found,
                    None => return Ok(()),
                }
            }
        };

        /* uninstall any previous, old versions */
        let all_installed = &self.get_installed();
        let already_installed = all_installed
            .iter()
            .find(|x| x.product_name.to_lowercase() == search.product_name.to_lowercase());
        if let Some(already) = already_installed {
            if already.version == actual_candidate.version {
                eprintln!(
                    "This version ({}) of the product is already installed on machine. Skipping.",
                    already.version
                );
                return Ok(());
            }
            eprintln!("Product already installed on machine. Uninstalling before continuing...");
            already.uninstall()?;
            eprintln!("Successfully Uninstalled product, continuing with new installation");
        }

        /* Launch installer */
        let binary_path = actual_candidate.make_output_for_candidate(&self.config.cache_directory);
        actual_candidate.install(&binary_path)
    }

    pub fn list_cache(&self) -> Option<Vec<InstallationCandidate>> {
        log::debug!(
            "Listing contents of cache directory {}",
            &self.config.cache_directory.to_str().unwrap()
        );
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

        log::debug!("Found {} cached items", found_candidates.len());

        /* Sort the candidates, in preference of Flavor, Version, Identifier */
        found_candidates.sort_by(|a, b| {
            let cmp_flavor = a.flavor.id.cmp(&b.flavor.id);

            if cmp_flavor == std::cmp::Ordering::Equal {
                let cmp_version = b
                    .version
                    .partial_cmp(&a.version)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if cmp_version == std::cmp::Ordering::Equal {
                    a.identifier.cmp(&b.identifier)
                } else {
                    cmp_version
                }
            } else {
                cmp_flavor
            }
        });

        Some(found_candidates)
    }

    /// Attempts to locate the installer for the candiate in the local cache
    fn locate_in_cache(&self, search: &SearchCandidate) -> Option<InstallationCandidate> {
        let mut found_candidates: Vec<InstallationCandidate> = self.list_cache()?;

        /* Drop non platform, non product items, non desired flavor items */
        found_candidates.retain(|x| {
            (x.flavor.platform == search.flavor.platform)
                && (x.product_name.to_lowercase() == search.product_name.to_lowercase()
                    && x.flavor.id.to_lowercase() == search.flavor.id.to_lowercase())
        });

        for found in found_candidates.into_iter() {
            /* if version is specified, that overrides everything, grab first matching one */
            if let Some(v) = &search.version {
                if v.to_lowercase() == found.version.to_lowercase() {
                    log::info!("Found exact version match in cache");
                    return Some(found);
                }
                /* Version wasnt a match, but version is mandatory. Skip. */
                continue;
            }
            if let Some(i) = &search.identifier {
                if i.to_lowercase() == found.identifier.to_lowercase() {
                    log::info!("Found matching identifier in cache");
                    return Some(found);
                }
                /* Identifier wasnt a match, but identifier is mandatory. Skip */
                continue;
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
        {
            let candidates = self
                .get_installed_mac()
                .expect("Failed to get installed gravio items");
            candidates
        }
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {}
    }

    #[cfg(target_os = "macos")]
    fn get_installed_mac(&self) -> Result<Vec<InstalledProduct>, Box<dyn std::error::Error>> {
        use std::collections::HashMap;

        use crate::product::PackageType;

        let mut installed: Vec<InstalledProduct> = Vec::new();
        /* list contents of /Applications */
        match fs::read_dir("/Applications") {
            Ok(list_dir) => {
                for entry_result in list_dir {
                    if let Ok(entry) = entry_result {
                        let path = entry.path();
                        if entry.file_type()?.is_dir() {
                            let app_path = path.join("Contents").join("Info.plist");
                            match plist::from_file::<std::path::PathBuf, HashMap<String, plist::Value>>(app_path.clone()) {
                                Ok(pl) => {
                                    let id = pl.get("CFBundleIdentifier");
                                    let exe_name = pl.get("CFBundleExecutable");
                                    let version_major_minor = pl.get("CFBundleShortVersionString");
                                    let version_build = pl.get("CFBundleVersion");
                                    if id.is_none() || exe_name.is_none() || version_major_minor.is_none() || version_build.is_none(){
                                        log::error!("Opened plist file but didnt have CFBundleIdentifier, CFBundleExecutable,nCFBundleShortVersionString, or CFBundleVersion  keys");
                                        continue;
                                    }
                                    let id = id.unwrap().as_string();
                                    let exe_name = exe_name.unwrap().as_string();
                                    let version_major_minor = version_major_minor.unwrap().as_string();
                                    let version_build = version_build.unwrap().as_string();
                                    if id.is_none() || exe_name.is_none() || version_major_minor.is_none() || version_build.is_none(){
                                        log::error!("CFBundleIdentifier or CDBundleExecutable were not strings");
                                        continue;
                                    }
                                    let found_id = id.unwrap();
                                    let found_exe_name = exe_name.unwrap();
                                    let found_version_major_minor = version_major_minor.unwrap();
                                    let found_version_build = version_build.unwrap();


                                    let mut product_name: String = String::default();
                                    let mut product_identifier: String = String::default();
                                    for product in &self.config.products {
                                        for flavor in &product.flavors {
                                            if flavor.platform == Platform::Mac {
                                                if let Some(metadata) = &flavor.metadata {
                                                    if let Some(known_id) = metadata.get("CFBundleIdentifier") {
                                                        if known_id == found_id {
                                                            product_identifier = known_id.into();
                                                            product_name = product.name.to_owned();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // &self.config.products.iter().find(|x|x.flavors.iter().find(|y|y.platform == Platform::Mac).)

                                    if product_identifier != String::default() {
                                        let instaled_product = InstalledProduct{
                                            product_name: product_name,
                                            version: Version::new(&format!("{}.{}", found_version_major_minor, found_version_build)),
                                            package_name: product_identifier,
                                            package_type: PackageType::Dmg,
                                        };

                                        installed.push(instaled_product);
                                    }

                                }
                                Err(e) => {
                                    log::error!("Failed to read contents of {}: {e}", &app_path.to_str().unwrap())
                                }
                            }
                        } 
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read /Applications directory: {}", e);
                return Err(Box::new(e));
            }
        };
        Ok(installed)
    }

    #[cfg(target_os = "windows")]
    fn get_installed_windows(&self) -> Result<Vec<InstalledProduct>, Box<dyn std::error::Error>> {
        let mut installed: Vec<InstalledProduct> = Vec::new();

        let publisher_ids_for_platform = self
            .config
            .publisher_identities
            .iter()
            .filter(|x| x.platforms.contains(&Platform::Windows))
            .map(|x| x.id.as_ref())
            .collect::<Vec<&str>>();

        if publisher_ids_for_platform.is_empty() {
            log::warn!("No publishers specified, therefore cant get any Windows installed application information");
            return Ok(installed);
        }

        /* get Appx Packages */
        {
            let publisher_where = publisher_ids_for_platform
                .iter()
                .map(|x| format!("$_.Publisher -eq \"{}\"", x))
                .collect::<Vec<String>>()
                .join(" -or ");

            let command = format!(
                "Get-AppxPackage | Where-Object {{{}}} | Select Name, Version, PackageFullName | ConvertTo-Json -Compress",
                publisher_where
            );
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(command)
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                // Convert the output bytes to a string
                let mut result = String::from_utf8_lossy(&output.stdout)
                    .to_owned()
                    .trim()
                    .to_string();
                if !(result.starts_with('[') && result.ends_with(']')) {
                    result.insert(0, '[');
                    result.push(']');
                };
                let v: Vec<InstalledAppXProduct> = serde_json::from_str(&result)?;
                for appx in v {
                    installed.push(appx.into());
                }
            } else {
                // Print the error message if the command failed
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(GManError::new(
                    "Failed to get installations: AppX items",
                )));
            }
        }

        /* get MSI installed items */
        {
            let publisher_where = publisher_ids_for_platform
                .iter()
                .map(|x| format!("$publisher -eq \"{}\"", x))
                .collect::<Vec<String>>()
                .join(" -or ");

            let command = {
                let parts = [
                    r#"foreach($obj in Get-ChildItem "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall") {
                    $dn = $obj.GetValue('DisplayName')
                    $publisher = $obj.GetValue('Publisher')
                    if($dn -ne $null -and ("#,
                    &publisher_where,
                    r#")) {
                        $key_name = ($obj | Select-Object Name | Split-Path -Leaf).replace('}}', '}')
                        $ver = $obj.GetValue('DisplayVersion')
                        Write-Host $dn@$ver@$key_name
                      }
                    }"#,
                ];

                String::from_iter(parts)
            };

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
                    let version = hubkit_splits[1].trim();
                    let identifier = hubkit_splits[2].trim().to_owned();

                    let installed_product = InstalledProduct {
                        product_name: product::PRODUCT_GRAVIO_HUBKIT.name.to_owned(),
                        version: Version::new(version),
                        package_name: identifier.to_owned(),
                        package_type: product::PackageType::Msi,
                    };

                    installed.push(installed_product);
                }
            } else {
                // Print the error message if the command failed
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(GManError::new(
                    "Failed to get installations: MSI items",
                )));
            }
        }

        /* get Gravio Sensor Map */
        {}

        Ok(installed)
    }

    pub fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = &self.config.cache_directory;
        log::debug!("Clearing cache directory {}", &path.to_str().unwrap());
        util::remove_dir_contents(path)
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

    use crate::{candidate::SearchCandidate, cli::Target, product, team_city, Client};

    #[tokio::test]
    async fn tets_candidates() {
        let client = Client::load().expect("Failed to load client");
        let candidates = client.list_candidates(None, None).await.unwrap();
        assert!(!candidates.is_empty());
        println!("lmao");
    }

    #[test]
    fn test_get_installed() {
        let client = Client::load().expect("Failed to load client");
        let installed = client.get_installed();
        assert!(!installed.is_empty())
    }

    #[tokio::test]
    async fn test_install_with_cache() {
        let p = &product::PRODUCT_GRAVIO_STUDIO;
        let client = Client::load().expect("Failed to load client");

        let search = SearchCandidate::new(
            &p.name,
            None,
            Some("develop"),
            None,
            &client.config.products,
        )
        .unwrap();
        let res = client.install(&search, None).await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_install_force_with_cache() {
        let p = &product::PRODUCT_GRAVIO_STUDIO;
        let client = Client::load().expect("Failed to load client");

        let search = SearchCandidate::new(
            &p.name,
            None,
            Some("develop"),
            None,
            &client.config.products,
        )
        .unwrap();

        let res = client.install(&search, Some(true)).await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_get_build_id_specific_version() {
        let p = &product::PRODUCT_GRAVIO_HUBKIT;

        let client = Client::load().expect("Failed to load client");

        let candidate = SearchCandidate::new(
            &p.name,
            Some("5.2.0-7015"),
            None,
            Some("WindowsHubkit"),
            &client.config.products,
        )
        .unwrap();

        let vv = client.get_valid_repositories_for_platform();

        match team_city::get_with_build_id_by_candidate(&client.http_client, &candidate, &vv).await
        {
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
        let p = &product::PRODUCT_GRAVIO_HUBKIT;
        let client = Client::load().expect("Failed to load client");

        let candidate = SearchCandidate::new(
            &p.name,
            None,
            Some("develop"),
            None,
            &client.config.products,
        )
        .unwrap();

        let vv = client.get_valid_repositories_for_platform();

        match team_city::get_with_build_id_by_candidate(&client.http_client, &candidate, &vv).await
        {
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
    async fn get_build_id_by_version() {
        let p = &product::PRODUCT_HANDBOOK_X;

        let client = Client::load().expect("Failed to load client");

        let candidate = SearchCandidate::new(
            &p.name,
            Some("1.0.1656.0"),
            None,
            Some("Windows"),
            &client.config.products,
        )
        .unwrap();

        let vv = client.get_valid_repositories_for_platform();

        match team_city::get_with_build_id_by_candidate(&client.http_client, &candidate, &vv).await
        {
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
        let p = &product::PRODUCT_GRAVIO_HUBKIT;

        let client = Client::load().expect("Failed to load client");

        let candidate = SearchCandidate::new(
            &p.name,
            None,
            Some("1a361e15-27e2-48b1-bc8b-054d9ab8c435"),
            None,
            &client.config.products,
        )
        .unwrap();

        let vv = client.get_valid_repositories_for_platform();

        match team_city::get_with_build_id_by_candidate(&client.http_client, &candidate, &vv).await
        {
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
        let client = Client::load().expect("Failed to load client");
        let target: Target = Target::Identifier("lmao".to_owned());

        let candidate = SearchCandidate::new(
            &product::PRODUCT_GRAVIO_HUBKIT.name,
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(x) => Some(x.as_str()),
            },
            None,
            &client.config.products,
        )
        .unwrap();

        client
            .install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_hubkit_develop() {
        let client = Client::load().expect("Failed to load client");
        let target: Target = Target::Identifier("develop".to_owned());

        let candidate = SearchCandidate::new(
            &product::PRODUCT_GRAVIO_HUBKIT.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
            &client.config.products,
        )
        .unwrap();

        client
            .install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_hubkit_specific_version() {
        let client = Client::load().expect("Failed to load client");
        let target: Target = Target::Version("5.2.1-7049".to_owned());

        let candidate = SearchCandidate::new(
            &product::PRODUCT_GRAVIO_HUBKIT.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
            &client.config.products,
        )
        .unwrap();

        client
            .install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_studio_specific_version() {
        let client = Client::load().expect("Failed to load client");
        let target: Target = Target::Version("5.2.4683".to_owned());

        let candidate = SearchCandidate::new(
            &product::PRODUCT_GRAVIO_STUDIO.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
            &client.config.products,
        )
        .unwrap();

        client
            .install(&candidate, Some(false))
            .await
            .expect("Failed to install item");
    }

    #[tokio::test]
    async fn install_handbookx_specific_version() {
        let client = Client::load().expect("Failed to load client");
        let target: Target = Target::Version("1.0.1656.0".to_owned());

        let candidate = SearchCandidate::new(
            &product::PRODUCT_HANDBOOK_X.name,
            match &target {
                Target::Identifier(_) => None,
                Target::Version(x) => Some(x.as_str()),
            },
            match &target {
                Target::Identifier(x) => Some(x.as_str()),
                Target::Version(_) => None,
            },
            None,
            &client.config.products,
        )
        .unwrap();

        client
            .install(&candidate, Some(false))
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

        let val = serde_json::from_str::<team_city::TeamCityArtifacts>(r);
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

        let val = serde_json::from_str::<team_city::TeamCityBuild>(r);
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

        let val = serde_json::from_str::<team_city::TeamCityBuilds>(r);
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

        let val = serde_json::from_str::<team_city::TeamCityBranch>(r);
        println!("{:#?}", val);
        assert!(val.is_ok());
    }

    #[tokio::test]
    async fn download_develop_hubkit() {
        let client = Client::load().expect("Failed to load client");
        let vv = client.get_valid_repositories_for_platform();
        let p = &product::PRODUCT_GRAVIO_HUBKIT;

        let c = SearchCandidate::new(
            &p.name,
            None,
            Some("develop"),
            None,
            &client.config.products,
        )
        .unwrap();

        let with_build_id = team_city::get_with_build_id_by_candidate(&client.http_client, &c, &vv)
            .await
            .expect("expected to get build id during test for develop hubkit install")
            .expect("Expected build id to exist");

        let _ = team_city::download_artifact(
            &client.http_client,
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
