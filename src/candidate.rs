use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use tabled::Tabled;
use tokio::fs;

use crate::{
    app,
    gman_error::GravioError,
    platform::Platform,
    product::{self, Flavor, Product},
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
            version: self.version.to_owned(),
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

    pub version: Option<String>,

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
            version: version.map(|x| x.to_owned()),
            identifier: identifier.map(|x| x.to_owned()),
            flavor: flavor_str.unwrap().to_owned(),
        })
    }

    pub fn version_or_identifier_string(&self) -> &str {
        if let Some(v) = &self.version {
            v.as_str()
        } else if let Some(i) = &self.identifier {
            i.as_str()
        } else {
            ""
        }
    }
}

#[derive(Debug)]
pub struct InstallationCandidate {
    pub remote_id: String,

    pub repo_location: String,

    pub product_name: String,

    pub version: String,

    pub identifier: String,

    pub flavor: Flavor,

    pub installed: bool,
}

impl InstallationCandidate {
    /// Some version strings, such as with gs/win, are 3-parts, but we often need to reference them by a 4-part scheme
    ///
    /// e.g, 5.2.7033 -> 5.3.7033.0
    fn make_version_4_parts(&self) -> String {
        let mut s = self.version.to_owned();
        let mut count = self.version.split('.').count();
        while count < 3 {
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
        /* Try Gravio Studio */
        if &self.product_name.to_lowercase() == &product::PRODUCT_GRAVIO_STUDIO.name.to_lowercase()
        {
            #[cfg(target_os = "windows")]
            {
                log::debug!("Creating a temporary file for this gs/win extraction");

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
                        "Failed to extract gs/win zip items: {}",
                        unzip_output.status.code().unwrap()
                    );
                    return Err(Box::new(GravioError::new(
                        "Failed to install Gravio Studio, couldn't extract to temp directory",
                    )));
                }

                /* run the  Install.ps1 */
                match std::fs::read_dir(tmp_folder) {
                    Ok(list_dir) => {
                        for entry_result in list_dir {
                            if let Ok(entry) = entry_result {
                                if entry.metadata().unwrap().is_dir() {
                                    let install_script_loc = entry.path().join("Install.ps1");
                                    if Path::exists(&install_script_loc) {
                                        log::debug!("found gs/win install.ps1 file");
                                        let install_output = Command::new("powershell")
                                            .arg("-Command")
                                            .arg(install_script_loc.to_str().unwrap())
                                            .output()?;

                                        if !install_output.status.success() {
                                            // Convert the output bytes to a string
                                            log::debug!(
                                                "Failed to install gs/win: {}",
                                                install_output.status.code().unwrap()
                                            );
                                            return Err(Box::new(GravioError::new(
                                                    "Failed to install Gravio Studio, couldn't run install script successfully",
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
                        return Err(Box::new(GravioError::new(
                            "Failed to read temporary extracted directory",
                        )));
                    }
                }
            }
        }
        /* Try HubKit */
        else if &self.product_name.to_lowercase()
            == &product::PRODUCT_GRAVIO_HUBKIT.name.to_lowercase()
        {
            #[cfg(target_os = "windows")]
            {
                let output = Command::new("msiexec")
                    .args(["/i", binary_path.to_str().unwrap()])
                    .output()?;

                // Check if the command was successful
                if output.status.success() {
                    // Convert the output bytes to a string
                    log::debug!("Successfully installed HubKit");
                    return Ok(());
                }
                if output.status.code().unwrap_or_default() == 1602 {
                    return Err(Box::new(GravioError::new("User canceled installation")));
                }
                return Err(Box::new(GravioError::new(
                    "Unknown error occurred during installation",
                )));
            }
        }
        Ok(())
    }
}

impl FromStr for InstallationCandidate {
    type Err = GravioError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splits = s.split('@').collect::<Vec<_>>();
        if splits.len() != 6 {
            return Err(GravioError::new("Not an InstallationCandidate string"));
        }
        let product_name = splits[0];
        let flavor_str = splits[2];
        let identifier = splits[3];
        let version = splits[4];

        let product = Product::from_name(product_name);
        if let None = product {
            return Err(GravioError::new(
                "Failed to extract product from InstallationCandidate FromStr",
            ));
        }
        let product = product.unwrap();

        let flavor = product
            .flavors
            .iter()
            .find(|x| x.name.to_lowercase() == flavor_str.to_lowercase());

        if let None = flavor {
            return Err(GravioError::new(
                "Failed to extract flavor from InstallationCandidate FromStr",
            ));
        }

        let c = Self {
            remote_id: String::default(),
            repo_location: String::default(),
            product_name: product_name.to_owned(),
            version: version.to_owned(),
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
}

impl InstalledProduct {
    pub fn uninstall(&self) -> Result<(), Box<dyn std::error::Error>> {
        /* Try Gravio Studio (win) */
        if &self.product_name == &product::PRODUCT_GRAVIO_STUDIO.name {
            #[cfg(target_os = "windows")]
            {
                let command = format!("Remove-AppxPackage {}", self.package_name);
                let output = Command::new("powershell")
                    .arg("-Command")
                    .arg(command)
                    .output()?;

                // Check if the command was successful
                if output.status.success() {
                    // Convert the output bytes to a string
                    log::debug!("Successfully uninstalled gs/win");
                    return Ok(());
                }
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(GravioError::new(
                    "Failed to get installations: Studio",
                )));
            }
        }
        /* Try HubKit */
        if &self.product_name == &product::PRODUCT_GRAVIO_HUBKIT.name {
            #[cfg(target_os = "windows")]
            {
                let output = Command::new("msiexec")
                    .args(["/x", self.package_name.as_str()])
                    .output()?;

                // Check if the command was successful
                if output.status.success() {
                    // Convert the output bytes to a string
                    log::debug!("Successfully uninstalled HubKit");
                    return Ok(());
                }
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(GravioError::new(
                    "Failed to get installations: Studio",
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::product;

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
            version: "5.2.3-7023".to_owned(),
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