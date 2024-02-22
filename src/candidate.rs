use std::process::Command;

use tabled::{Table, Tabled};

use crate::{
    gman_error::MyError,
    product::{self, Product},
};

#[derive(Debug, Tabled)]
pub struct Candidate<'a> {
    /// internal id, often just a TeamCity reference
    #[tabled(skip)]
    pub remote_id: Option<String>,

    /// Display name of the Candidate, usually a Product
    #[tabled(order = 0)]
    pub name: String,

    #[tabled(skip)]
    pub product: &'a Product,

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

impl PartialEq for Candidate<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

impl Candidate<'_> {
    pub fn uninstall(&self) -> Result<(), Box<dyn std::error::Error>> {
        /* Try Gravio Studio (win) */
        #[cfg(target_os = "windows")]
        if &self.product.name == &product::PRODUCT_GRAVIO_STUDIO_WINDOWS.name
            && &self.product.teamcity_id == &product::PRODUCT_GRAVIO_STUDIO_WINDOWS.teamcity_id
        {
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
