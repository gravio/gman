use fs_extra::dir;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use regex::Regex;
use serde::Deserialize;
use std::{
    fmt::Display, ops::Deref, path::{Path, PathBuf}, process::Command, str::FromStr
};

use tabled::Tabled;

use crate::{
    app,
    gman_error::GManError,
    platform::Platform,
    product::{self, Flavor, PackageType, Product},
};
use lazy_static::lazy_static;

#[derive(Tabled)]
pub struct TablePrinter {
    #[tabled(order = 0)]
    pub name: String,
    #[tabled(order = 1)]
    pub version: String,
    #[tabled(order = 2)]
    pub identifier: String,
    #[tabled(order = 3)]
    pub flavor: String,
    #[tabled(order = 4)]
    pub installed: bool,
}

impl Into<TablePrinter> for InstallationCandidate {
    fn into(self) -> TablePrinter {
        TablePrinter {
            identifier: self.identifier.to_owned(),
            name: self.product_name.to_owned(),
            version: self.version.into(),
            flavor: self.flavor.id.to_owned(),
            installed: self.installed,
        }
    }
}

impl Into<TablePrinter> for InstalledProduct {
    fn into(self) -> TablePrinter {
        TablePrinter {
            identifier: self.package_name.to_owned(),
            name: self.product_name.to_owned(),
            version: self.version.into(),
            flavor: String::default(),
            installed: true,
        }
    }
}

impl From<&InstalledProduct> for TablePrinter {
    fn from(value: &InstalledProduct) -> Self {
        TablePrinter {
            identifier: value.package_name.to_owned(),
            name: value.product_name.to_owned(),
            version: value.version.0.to_owned(),
            flavor: String::default(),
            installed: true,
        }
    }
}

pub struct SearchCandidate {
    pub product_name: String,

    pub version: Option<Version>,

    pub identifier: Option<String>,

    pub flavor: Flavor,
}

impl SearchCandidate {
    pub fn new(
        product_name: &str,
        version: Option<&str>,
        identifier: Option<&str>,
        flavor: Option<&str>,
        available_products: &Vec<Product>,
    ) -> Option<SearchCandidate> {
        let product_lower = product_name.to_lowercase();
        let product = match available_products
            .iter()
            .find(|m| m.name.to_lowercase() == product_lower)
        {
            Some(p) => p,
            None => return None,
        };

        let current_platform = Platform::platform_for_current_platform().unwrap();
        let flavor_str = match flavor {
            Some(f_str) => {
                let flavor_lower = f_str.to_lowercase();
                product
                    .flavors
                    .iter()
                    .find(|x| x.id.to_lowercase() == flavor_lower)
            }
            None => product
                .flavors
                .iter()
                .find(|x| x.platform == current_platform),
        };

        if flavor_str.is_none() {
            eprintln!("No flavor found, not even default");
            return None;
        }

        Some(SearchCandidate {
            product_name: product_name.to_owned(),
            version: version.map(|x| Version::new(x)),
            identifier: identifier.map(|x| x.to_owned()),
            flavor: flavor_str.unwrap().to_owned(),
        })
    }

    pub fn version_or_identifier_string(&self) -> &str {
        if let Some(v) = &self.version {
            &v
        } else if let Some(i) = &self.identifier {
            i.as_str()
        } else {
            ""
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Version(String);

impl Version {
    pub fn new(version_str: &str) -> Self {
        Self(version_str.to_owned())
    }

    pub fn make_version_4_parts(&self) -> Version {
        let mut s = self.0.to_owned();
        let mut count = s.split('.').count();
        while count < 4 {
            count += 1;
            s.push_str(".0");
        }
        Version::new(&s)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.make_version_4_parts().0 == other.make_version_4_parts().0
    }
}

impl Eq for Version {}

lazy_static! {
    static ref MOUNTED_VOLUME_REGEX: Regex =
        Regex::new(r"(/Volumes/.+$)").expect("Failed to create Volumes regex");
    static ref VERSION_REGEX: Regex =
        Regex::new(r#"^(\d{1,})(?:[.-](\d{1,}))?(?:[.-](\d{1,}))?(?:[.-](\d{1,}))?$"#)
            .expect("Failed to create Version 1 regex");
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let caps_self: Vec<&str> = match VERSION_REGEX.captures(&self.0) {
            Some(c) => c,
            None => return None,
        }
        .iter()
        .skip(1)
        .filter_map(|m| m.map(|m| m.as_str()))
        .collect();

        let caps_other: Vec<&str> = match VERSION_REGEX.captures(&other.0) {
            Some(c) => c,
            None => return None,
        }
        .iter()
        .skip(1)
        .filter_map(|m| m.map(|m| m.as_str()))
        .collect();

        for zipped in caps_self.iter().zip(caps_other.iter()) {
            let z0 = u32::from_str(zipped.0).unwrap();
            let z1 = u32::from_str(zipped.1).unwrap();

            let cmp = z0.cmp(&z1);
            if cmp != std::cmp::Ordering::Equal {
                return Some(cmp);
            }
        }

        Some(std::cmp::Ordering::Equal)
    }
}

impl Deref for Version {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        &self.0.as_ref()
    }
}

impl Into<String> for Version {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug)]
pub struct InstallationCandidate {
    pub remote_id: String,

    pub repo_location: String,

    pub product_name: String,

    pub version: Version,

    pub identifier: String,

    pub flavor: Flavor,

    pub installed: bool,
}

#[cfg(target_os = "macos")]
const MAC_APPLICATIONS_DIR: &'static str = "/Applications";

impl InstallationCandidate {
    pub fn product_equals(&self, installed_product: &InstalledProduct) -> bool {
        &installed_product.product_name == &self.product_name
    }

    /// Returns the file name of the file this InstallationCandidate represents
    pub fn get_binary_file_name(&self) -> String {
        PathBuf::from_str(&self.flavor.teamcity_metadata.teamcity_binary_path)
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    /// Makes a file name for the InstallationCandidate, encoding the the necessary info to make lookups easy
    ///
    /// format is "product_name@platform@flavor_name@identifier@version@binary_name"
    /// e.g., "graviostudio@windows@sideloading@develop@5.2.1-7033@GravioStudio.msi"
    pub fn make_cached_file_name(&self) -> String {
        format!(
            "{}@{}@{}@{}@{}@{}",
            &self.product_name,
            &self.flavor.platform,
            &self.flavor.id,
            &self.identifier,
            &self.version,
            &self.get_binary_file_name()
        )
    }

    /// Gets the path of the file that the InstallationCandidate downloads to on disk
    pub fn make_output_for_candidate(&self, dir: &Path) -> PathBuf {
        let fname = &self.make_cached_file_name();
        dir.join(fname)
    }

    pub fn install(&self, binary_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        /* Try UWP */
        #[cfg(target_os = "windows")]
        if self.flavor.package_type == PackageType::AppX {
            log::debug!("Creating a temporary file for this appx extraction");

            let tmp_folder = app::get_app_temp_directory().join(self.make_cached_file_name());
            std::fs::create_dir_all(&tmp_folder)?;

            let unzip_command = format!(
                "Expand-Archive \"{}\" \"{}\" -force",
                &binary_path.to_str().unwrap(),
                &tmp_folder.to_str().unwrap()
            );
            /* extract zip to temporary directory */
            log::debug!("Sending extract-archive request to powershell");
            let unzip_output = Command::new("powershell")
                .arg("-Command")
                .arg(unzip_command)
                .output()?;

            if !unzip_output.status.success() {
                // Convert the output bytes to a string
                log::debug!(
                    "Failed to extract appx zip items: {}",
                    unzip_output.status.code().unwrap()
                );
                return Err(Box::new(GManError::new(&format!(
                    "Failed to install {}, couldn't extract to temp directory",
                    self.product_name
                ))));
            }

            /* run the  Install.ps1 */
            match std::fs::read_dir(tmp_folder) {
                Ok(list_dir) => {
                    for entry_result in list_dir {
                        if let Ok(entry) = entry_result {
                            if entry.metadata().unwrap().is_dir() {
                                let install_script_loc = entry.path().join("Install.ps1");
                                if Path::exists(&install_script_loc) {
                                    log::debug!("found {} install.ps1 file", self.product_name);
                                    let install_output = Command::new("powershell")
                                        .arg("-Command")
                                        .arg(install_script_loc.to_str().unwrap())
                                        .output()?;

                                    if !install_output.status.success() {
                                        log::debug!(
                                            "Failed to install {}: {}",
                                            self.product_name,
                                            install_output.status.code().unwrap()
                                        );
                                        return Err(Box::new(GManError::new(
                                                    &format!("Failed to install {}, couldn't run install script successfully", self.product_name),
                                                )));
                                    }
                                    return Ok(());
                                }
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    log::error!("Failed to read temporary extracted directory");
                    return Err(Box::new(GManError::new(
                        "Failed to read temporary extracted directory",
                    )));
                }
            }
        }
        /* Try misx */
        else if self.flavor.package_type == PackageType::MsiX {
            let install_command = format!("Add-AppxPackage \"{}\"", binary_path.to_str().unwrap());
            let install_output = Command::new("powershell")
                .arg("-Command")
                .arg(install_command)
                .output()?;

            if !install_output.status.success() {
                // Convert the output bytes to a string
                log::debug!(
                    "Failed to install {}: {}",
                    self.product_name,
                    install_output.status.code().unwrap()
                );
                return Err(Box::new(GManError::new(&format!(
                    "Failed to install {}, couldn't run MSIX installer successfully",
                    self.product_name
                ))));
            }
        } else if self.flavor.package_type == PackageType::Msi {
            let output = Command::new("msiexec")
                .args(["/i", binary_path.to_str().unwrap()])
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                // Convert the output bytes to a string
                log::debug!("Successfully installed {}", self.product_name);
                return Ok(());
            }
            if output.status.code().unwrap_or_default() == 1602 {
                return Err(Box::new(GManError::new("User canceled installation")));
            }
            return Err(Box::new(GManError::new(
                "Unknown error occurred during installation",
            )));
        }

        #[cfg(target_os = "macos")]
        {
            return install_mac(binary_path);
        }

        #[cfg(target_os = "linux")]
        {}
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn install_mac(binary_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    /* make temporary folder on system */

    use std::fmt::Write;
    let temp_dir = {
        let output = Command::new("mktemp").arg("-d").output()?;

        // Check if the command was successful
        if output.status.success() {
            // Convert the output bytes to a string
            let result = String::from_utf8_lossy(&output.stdout);
            let result = result.to_string();
            log::debug!("Successfully made temporary directory: {}", &result);
            result.to_owned()
        } else {
            return Err(Box::new(GManError::new(
                "Unknown error occurred while making temporary folder",
            )));
        }
    };

    /* mount the dmg file */
    let mount = {
        let output = Command::new("hdiutil")
            .arg("attach")
            .arg(binary_path)
            .output()?;

        // Check if the command was successful
        if output.status.success() {
            log::debug!("Successfully mounted dmg file");
            // Convert the output bytes to a string
            let result = String::from_utf8_lossy(&output.stdout);
            let lines = result.split('\n');

            let mut mount_point: Option<String> = None;
            for line in lines {
                let trimmed = line.trim();
                let caps_volume: Vec<&str> = match MOUNTED_VOLUME_REGEX.captures(trimmed) {
                    Some(c) => c,
                    None => {
                        continue;
                    }
                }
                .iter()
                .skip(1)
                .filter_map(|m| m.map(|m| m.as_str()))
                .collect();
                mount_point = Some(caps_volume.first().unwrap().to_string());
                break;
            }
            mount_point
        } else {
            return Err(Box::new(GManError::new(
                "Unknown error occurred while making temporary folder",
            )));
        }
    };

    match mount {
        Some(volume) => {
            log::info!("Got mount point for application: {}", volume.as_str());
            log::info!("Checking if mounted contents are .app or .pkg");

            let is_dot_app: Option<MacPackage> = {
                let output = Command::new("ls").arg(&volume).output()?;
                if output.status.success() {
                    log::debug!("ls'd mounted volume");
                    let result = String::from_utf8_lossy(&output.stdout);
                    let lines = result.split('\n').collect::<Vec<&str>>();
                    let found_app = lines.iter().find(|x| x.ends_with(".app"));
                    match found_app {
                        Some(app_path) => {
                            let full_path = PathBuf::from_str(&volume).unwrap().join(app_path);

                            Some(MacPackage {
                                is_app: true,
                                is_pkg: false,
                                path: full_path.to_str().unwrap().to_string(),
                            })
                        }
                        None => {
                            let found_pkg = lines.iter().find(|x| x.ends_with(".pkg"));
                            match found_pkg {
                                Some(app_path) => {
                                    let full_path =
                                        PathBuf::from_str(&volume).unwrap().join(app_path);
                                    Some(MacPackage {
                                        is_app: false,
                                        is_pkg: true,
                                        path: full_path.to_str().unwrap().to_string(),
                                    })
                                }
                                None => None,
                            }
                        }
                    }
                } else {
                    return Err(Box::new(GManError::new(&format!(
                        "Failed to ls mounted directory: {}",
                        output.status
                    ))));
                }
            };

            if let Some(package) = is_dot_app {
                if package.is_app {
                    log::debug!(
                        "Inner contents are .app, will copy directly  from {} to /Applications",
                        &package.path
                    );

                    let last_level = app::disable_logging();
                    let progress_bar = ProgressBar::new(0);
                    progress_bar.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                            .unwrap()
                            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                            .progress_chars("#>-"));

                    fs_extra::copy_items_with_progress(
                        &[package.path],
                        MAC_APPLICATIONS_DIR,
                        &dir::CopyOptions::new().overwrite(true),
                        |process_info| {
                            progress_bar.set_length(process_info.total_bytes);
                            progress_bar.set_position(process_info.copied_bytes);
                            fs_extra::dir::TransitProcessResult::ContinueOrAbort
                        }
                    )?;
                    app::enable_logging(last_level);

                    log::info!("Copied Application from mount to /Applications");
                } else if package.is_pkg {
                    log::debug!("Inner contensts are .pkg, will run dpkg installer");
                    let output = Command::new("installer")
                        .arg("-pkg")
                        .arg(&volume)
                        .arg("-target")
                        .arg("/")
                        .output()?;

                    if output.status.success() {
                        log::debug!("Successfully ran installer for package contents");
                    } else {
                        log::error!(
                            "Failed to run installer for package contents: {}",
                            &output.status
                        );
                        return Err(Box::new(GManError::new(&format!(
                            "Failed to run installer for package contents: {}",
                            &output.status
                        ))));
                    }
                } else {
                    log::warn!("Mounted item but contents were neither app nor pkg");
                }
            } else {
                log::warn!("Mounted item but could not extract contents");
            }

            let output = Command::new("hdiutil")
                .arg("detach")
                .arg(&volume)
                .output()?;

            if output.status.success() {
                log::debug!("Unmounted volume at {}", volume);
            } else {
                log::error!("Failed to unmount volume at {}", &volume);
                return Err(Box::new(GManError::new(&format!(
                    "Failed to unmount volume at {}",
                    volume
                ))));
            }

            Ok(())
        }
        None => {
            log::error!("Failed to get mount point");
            Err(Box::new(GManError::new("Failed to get mount point")))
        }
    }
}

impl FromStr for InstallationCandidate {
    type Err = GManError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splits = s.split('@').collect::<Vec<_>>();
        if splits.len() != 6 {
            return Err(GManError::new("Not an InstallationCandidate string"));
        }
        let product_name = splits[0];
        let flavor_str = splits[2];
        let identifier = splits[3];
        let version = splits[4];

        let product = Product::from_name(product_name, &Vec::new());
        if let None = product {
            return Err(GManError::new(
                "Failed to extract product from InstallationCandidate FromStr",
            ));
        }
        let product = product.unwrap();

        let flavor = product
            .flavors
            .iter()
            .find(|x| x.id.to_lowercase() == flavor_str.to_lowercase());

        if let None = flavor {
            return Err(GManError::new(
                "Failed to extract flavor from InstallationCandidate FromStr",
            ));
        }

        let c = Self {
            remote_id: String::default(),
            repo_location: String::default(),
            product_name: product_name.to_owned(),
            version: Version::new(version),
            identifier: identifier.to_owned(),
            flavor: flavor.unwrap().to_owned(),
            installed: false,
        };

        Ok(c)
    }
}

#[derive(Debug)]
pub struct InstalledProduct {
    pub product_name: String,

    pub version: Version,

    pub package_name: String,
    pub package_type: PackageType,
}

#[cfg(target_os = "windows")]
impl From<InstalledAppXProduct> for InstalledProduct {
    fn from(value: InstalledAppXProduct) -> Self {
        InstalledProduct {
            product_name: value.name.split('.').last().unwrap().to_owned(),
            version: value.version,
            package_name: value.package_full_name,
            package_type: PackageType::AppX,
        }
    }
}

impl InstalledProduct {
    pub fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Shutting down {} if running", &self.product_name);

        #[cfg(target_os = "macos")]
        /* Shut down the running process, if any */
        shutdown_program_mac(&self)?;

        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Uninstalling {}", &self.product_name);
        #[cfg(target_os = "windows")]
        if self.package_type == PackageType::AppX {
            let command = format!("Remove-AppxPackage {}", self.package_name);
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(command)
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                // Convert the output bytes to a string
                log::debug!("Successfully uninstalled {}", self.product_name);
                return Ok(());
            }
            eprintln!("PowerShell command failed:\n{:?}", output.status);
            return Err(Box::new(GManError::new(&format!(
                "Failed to get installations: {}",
                self.product_name
            ))));
        } else if self.package_type == PackageType::Msi {
            let output = Command::new("msiexec")
                .args(["/x", self.package_name.as_str()])
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                // Convert the output bytes to a string
                log::debug!("Successfully uninstalled {}", self.product_name);
                return Ok(());
            }
            eprintln!("PowerShell command failed:\n{:?}", output.status);
            return Err(Box::new(GManError::new(&format!(
                "Failed to get installations: {}",
                self.product_name
            ))));
        }

        #[cfg(target_os = "macos")]
        {
            /* Move entry in /Applications to trash */
            if let Some(path) = get_path_to_application_mac(&self)? {
                log::debug!("Sending {} to trash", &path.to_str().unwrap());
                let output = Command::new("rm").arg("-r").arg(path).output()?;
                if output.status.success() {
                    log::debug!("Successfully removed Application to trash");
                    return Ok(());
                }
                return Err(Box::new(GManError::new(&format!(
                    "Failed to remove application from /Applications directory: {}",
                    output.status
                ))));
            }
        }
        #[cfg(target_os = "linux")]
        {}
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct MacPackage {
    is_pkg: bool,
    is_app: bool,
    path: String,
}

#[cfg(target_os = "macos")]
fn get_path_to_application_mac(
    installed: &InstalledProduct,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    use std::{collections::HashMap, fs};

    /* list contents of /Applications */
    match fs::read_dir(MAC_APPLICATIONS_DIR) {
        Ok(list_dir) => {
            for entry_result in list_dir {
                if let Ok(entry) = entry_result {
                    let path = entry.path();
                    if entry.file_type()?.is_dir() {
                        let app_path = path.join("Contents").join("Info.plist");
                        match plist::from_file::<std::path::PathBuf, HashMap<String, plist::Value>>(
                            app_path.clone(),
                        ) {
                            Ok(pl) => {
                                let id = pl.get("CFBundleIdentifier");
                                if id.is_none() {
                                    log::error!("Opened plist file but didnt have CFBundleIdentifier, CFBundleExecutable,nCFBundleShortVersionString, or CFBundleVersion  keys");
                                    continue;
                                }
                                let id = id.unwrap().as_string();
                                if id.is_none() {
                                    log::error!(
                                        "CFBundleIdentifier or CDBundleExecutable were not strings"
                                    );
                                    continue;
                                }
                                let found_id = id.unwrap();

                                if found_id == installed.package_name {
                                    let p = path;
                                    return Ok(Some(p.as_path().to_owned()));
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to read contents of {}: {e}",
                                    &app_path.to_str().unwrap()
                                );
                                continue;
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
    log::debug!("No entries known for this application, may already be uninstalled");
    Ok(None)
}

#[cfg(target_os = "macos")]
fn get_running_app_pids_mac() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    log::debug!("Getting running processes");
    let mut pid_labels: Vec<String> = Vec::new();

    let output = Command::new("launchctl").arg("list").output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        let lines = result.split('\n');
        for line in lines {
            let splits = line.split('\t').collect::<Vec<&str>>();
            if splits.len() > 2 {
                let label = splits[2];
                pid_labels.push(label.into());
            }
        }

        Ok(pid_labels)
    } else {
        Err(Box::new(GManError::new(
            "Couldnt get PIDs for determinng running applications",
        )))
    }
}

/// shuts down a program, usually by its Identifier.
/// This is the first step before Uninstalling
#[cfg(target_os = "macos")]
fn shutdown_program_mac(installed: &InstalledProduct) -> Result<(), Box<dyn std::error::Error>> {
    let running_processes = get_running_app_pids_mac()?;

    match running_processes
        .iter()
        .find(|x| x.contains(&installed.package_name))
    {
        Some(running) => {
            log::debug!("Stopping application {}", running.as_str());
            let output = Command::new("launchctl")
                .arg("stop")
                .arg(running.as_str())
                .output()?;

            // Check if the command was successful
            if output.status.success() {
                log::debug!("Successfully stopped application");
                Ok(())
            } else {
                log::error!("Failed to stop application: {}", output.status);
                Err(Box::new(GManError::new(&format!(
                    "Failed to kill process id {} for application {}: {}",
                    running.as_str(),
                    &installed.package_name,
                    &output.status,
                ))))
            }
        }
        None => {
            log::debug!(
                "Tried to stop running application {}, but not found in running pids list",
                &installed.package_name
            );
            Ok(())
        }
    }
}

#[cfg(windows)]
#[derive(Deserialize)]
pub struct InstalledAppXProduct {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: Version,
    #[serde(rename = "PackageFullName")]
    pub package_full_name: String,
}

#[cfg(test)]
mod tests {
    use crate::{candidate::Version, product};

    use super::InstallationCandidate;

    #[test]
    fn test_cached_file_name() {
        let i = InstallationCandidate {
            flavor: product::PRODUCT_GRAVIO_HUBKIT
                .flavors
                .first()
                .unwrap()
                .to_owned(),
            identifier: "develop".to_owned(),
            version: Version::new("5.2.3-7023"),
            product_name: product::PRODUCT_GRAVIO_HUBKIT.name.to_owned(),
            remote_id: String::default(),
            repo_location: String::default(),
            installed: false,
        };

        let fname = i.make_cached_file_name();
        assert_eq!(
            fname,
            "HubKit@Windows@WindowsHubkit@develop@5.2.3-7023@GravioHubKit.msi"
        );
    }

    #[test]
    fn test_version_cmp_greater_full() {
        let v0 = Version::new("5.2.0.2222");
        let v1 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Greater);

        let v0 = Version::new("5.2.1.0001");
        let v1 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Greater);

        let v0 = Version::new("5.3.0.0001");
        let v1 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Greater);

        let v0 = Version::new("6.2.0.2222");
        let v1 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Greater);

        let v0 = Version::new("6.2.0.2222");
        let v1 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_version_cmp_greater_half() {
        let v0 = Version::new("5.2.3");
        let v1 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_version_cmp_less_full() {
        let v1 = Version::new("5.2.0.2222");
        let v0 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Less);

        let v1 = Version::new("5.2.1.0001");
        let v0 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Less);

        let v1 = Version::new("5.3.0.0001");
        let v0 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Less);

        let v1 = Version::new("6.2.0.2222");
        let v0 = Version::new("5.2.0.0001");

        let o = v0.partial_cmp(&v1);
        assert_eq!(o.unwrap(), std::cmp::Ordering::Less);
    }
}
