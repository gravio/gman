use std::process::Command;

use tabled::{Table, Tabled};

use crate::{
    gman_error::MyError,
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
}

impl From<&InstallationCandidate> for TablePrinter {
    fn from(value: &InstallationCandidate) -> Self {
        TablePrinter {
            identifier: value.identifier.to_owned(),
            name: value.product_name.to_owned(),
            version: value.version.to_owned(),
            flavor: value.flavor.name.to_owned(),
        }
    }
}

impl Into<TablePrinter> for InstallationCandidate {
    fn into(self) -> TablePrinter {
        TablePrinter {
            identifier: self.identifier.to_owned(),
            name: self.product_name.to_owned(),
            version: self.version.to_owned(),
            flavor: self.flavor.name.to_owned(),
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
        }
    }
}

#[derive(Debug)]
pub struct InstallationCandidate {
    pub remote_id: String,

    pub product_name: String,

    pub version: String,

    pub identifier: String,

    pub flavor: Flavor,
}

impl InstallationCandidate {
    pub fn product_equals(&self, installed_product: &InstalledProduct) -> bool {
        &installed_product.product_name == &self.product_name
    }
}

#[derive(Debug)]
pub struct InstalledProduct {
    pub product_name: String,

    pub version: String,

    pub package_name: String,
}

impl InstalledProduct {
    pub fn candidate_equals(&self, candidate: &InstallationCandidate) -> bool {
        self.product_name == candidate.product_name
    }
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
                return Err(Box::new(MyError::new(
                    "Failed to get installations: Studio",
                )));
            }
        }
        /* Try HubKit */
        if &self.product_name == &product::PRODUCT_GRAVIO_HUBKIT.name {
            #[cfg(target_os = "windows")]
            {
                let command = format!("msiexec /x \"{}\"", self.package_name);
                let output = Command::new("powershell")
                    .arg("-Command")
                    .arg(command)
                    .output()?;

                // Check if the command was successful
                if output.status.success() {
                    // Convert the output bytes to a string
                    log::debug!("Successfully uninstalled HubKit");
                    return Ok(());
                }
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(MyError::new(
                    "Failed to get installations: Studio",
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Tabled)]
pub struct Candidate {
    /// internal id, often just a TeamCity reference
    #[tabled(skip)]
    pub remote_id: Option<String>,

    /// Display name of the Candidate, usually a Product
    #[tabled(order = 0)]
    pub name: String,

    #[tabled(skip)]
    pub product: Product,

    /// Description of what this Candidate is
    #[tabled(skip)]
    pub description: Option<String>,

    /// Version, such as 5.2.1-7033
    #[tabled(order = 1)]
    pub version: String,

    /// User friendly identifier, usually a Branch name, so a user may install by "master" or "qos_bugfix"
    #[tabled(order = 2)]
    pub identifier: String,

    /// Whetehr this candidate is installed to the user's machine
    #[tabled(skip)]
    pub installed: bool,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

impl Candidate {
    pub fn version_or_identifier_string(&self) -> &str {
        if (&self.version).is_empty() {
            &self.identifier
        } else {
            &self.version
        }
    }
    pub fn uninstall(&self) -> Result<(), Box<dyn std::error::Error>> {
        /* Try Gravio Studio (win) */
        #[cfg(target_os = "windows")]
        if &self.product.name == &product::PRODUCT_GRAVIO_STUDIO.name {
            let command = format!("Remove-AppxPackage {}", self.identifier);
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
            return Err(Box::new(MyError::new(
                "Failed to get installations: Studio",
            )));
        }
        /* Try HubKit */
        if &self.product.name == &product::PRODUCT_GRAVIO_HUBKIT.name {
            #[cfg(target_os = "windows")]
            {
                let command = format!("msiexec /x \"{}\"", self.identifier);
                let output = Command::new("powershell")
                    .arg("-Command")
                    .arg(command)
                    .output()?;

                // Check if the command was successful
                if output.status.success() {
                    // Convert the output bytes to a string
                    log::debug!("Successfully uninstalled HubKit");
                    return Ok(());
                }
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(MyError::new(
                    "Failed to get installations: Studio",
                )));
            }
        }
        Ok(())
    }
}
