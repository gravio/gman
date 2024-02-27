use serde::Deserialize;
use std::{
    fmt::Display,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use tabled::Tabled;

use crate::{
    app,
    gman_error::GManError,
    platform::Platform,
    product::{self, Flavor, PackageType, Product},
};

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
            flavor: self.flavor.name.to_owned(),
            installed: self.installed,
        }
    }
}

impl Into<TablePrinter> for InstalledProduct {
    fn into(self) -> TablePrinter {
        TablePrinter {
            identifier: self.package_name.to_owned(),
            name: self.product_name.to_owned(),
            version: self.version.to_owned(),
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
            version: value.version.to_owned(),
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
    ) -> Option<SearchCandidate> {
        let product_lower = product_name.to_lowercase();
        let product = match product::ALL_PRODUCTS
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
                    .find(|x| x.name.to_lowercase() == flavor_lower)
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

#[derive(Debug, Clone)]
pub struct Version(String);

impl Version {
    pub fn new(version_str: &str) -> Self {
        Self(version_str.to_owned())
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

impl InstallationCandidate {
    // pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {}
    /// Some version strings, such as with gs/win, are 3-parts, but we often need to reference them by a 4-part scheme
    ///
    /// e.g, 5.2.7033 -> 5.3.7033.0
    pub fn make_version_4_parts(&self) -> String {
        let mut s = self.version.0.to_owned();
        let mut count = self.version.split('.').count();
        while count < 4 {
            count += 1;
            s.push_str(".0");
        }
        s
    }
    pub fn product_equals(&self, installed_product: &InstalledProduct) -> bool {
        &installed_product.product_name == &self.product_name
    }

    /// Returns the file name of the file this InstallationCandidate represents
    pub fn get_binary_file_name(&self) -> String {
        PathBuf::from_str(&self.flavor.teamcity_executable_path)
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
            &self.flavor.name,
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
        {}

        #[cfg(target_os = "linux")]
        {}
        Ok(())
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

        let product = Product::from_name(product_name);
        if let None = product {
            return Err(GManError::new(
                "Failed to extract product from InstallationCandidate FromStr",
            ));
        }
        let product = product.unwrap();

        let flavor = product
            .flavors
            .iter()
            .find(|x| x.name.to_lowercase() == flavor_str.to_lowercase());

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

    pub version: String,

    pub package_name: String,

    pub package_type: PackageType,
}

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
    pub fn uninstall(&self) -> Result<(), Box<dyn std::error::Error>> {
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

        #[cfg(target_os = "maos")]
        {}

        #[cfg(target_os = "linux")]
        {}
        Ok(())
    }
}

#[cfg(windows)]
#[derive(Deserialize)]
pub struct InstalledAppXProduct {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: String,
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
}
