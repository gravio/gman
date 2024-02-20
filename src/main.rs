use std::{
    error::{self, Error},
    fmt,
    process::{exit, Command},
};

use clap::{Parser, Subcommand};
use simple_logger::SimpleLogger;

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::List) => {
            let c = Client::load().expect("Couldnt load client");
            let candidates = c
                .list_candidates(None, None)
                .expect("Failed to load candidates");
            c.format_candidate_table(&candidates);
            exit(0)
        }
        Some(Commands::Uninstall { name }) => {
            let c = Client::load().expect("Couldnt load client");
            print!("uninstalling an item: {:?}", name);
            exit(0)
        }
        Some(Commands::Install { name, ver }) => {
            let c = Client::load().expect("Couldnt load client");

            let version = match ver {
                Some(x) => x.to_owned(),
                None => "master".to_owned(),
            };
            println!("Installing {:#?}@{:#?}", name, version);

            exit(0)
        }
        Some(Commands::Installed) => {
            let c = Client::load().expect("Couldnt load client");
            c.get_installed();

            exit(0)
        }
        None => {
            println!("Default subcommand");
        }
    }
}

struct Client {
    config: ClientConfig,
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
        Ok(ClientConfig::new())
    }

    /// Lists the available candidates of Gravio items to install
    ///
    /// The list of candidates is retrieved from the repoository server defined in the [ClientConfig]
    fn list_candidates(
        &self,
        name: Option<&str>,
        version: Option<&str>,
    ) -> Result<Vec<Candidate>, Box<dyn std::error::Error>> {
        log::debug!(
            "Listing candidates: name: {:#?}, version: {:#?}",
            name,
            version
        );

        /* fake candidates */
        Ok(vec![
            Candidate {
                name: "Hubkit".to_owned(),
                description: None,
                version: "5.2.1-8033".to_owned(),
                identifier: "5.2.1".to_owned(),
                installed: false,
            },
            Candidate {
                name: "Hubkit".to_owned(),
                description: None,
                version: "develop (5.2.1-7023)".to_owned(),
                identifier: "develop".to_owned(),
                installed: false,
            },
            Candidate {
                name: "gs/win".to_owned(),
                description: None,
                version: "5.1.12-8831".to_owned(),
                identifier: "5.1".to_owned(),
                installed: false,
            },
        ])
    }

    /// Lists items installed to this machine
    fn get_installed(&self) {
        log::debug!("Getting installed Gravio items");
        #[cfg(target_os = "windows")]
        {
            let candidates = self
                .get_installed_windows()
                .expect("Failed to get installed gravio items");
            self.format_candidate_table(&candidates);
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
                    let name = "gs/win".to_owned();
                    let version = vec[1].trim().to_owned();
                    let location = vec[2].trim().to_owned();

                    let c = Candidate {
                        name,
                        version,
                        identifier: location,
                        description: None,
                        installed: true,
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
                  Write-Host $dn@$ver
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
                    let name = "Hubkit".to_owned();
                    let version = hubkit_splits[1].trim().to_owned();
                    let identifier = hubkit_splits[0].trim().to_owned();

                    let c = Candidate {
                        name,
                        version,
                        identifier,
                        description: None,
                        installed: true,
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
            let command = r#"
            foreach($obj in Get-ChildItem "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall") {
                $dn = $obj.GetValue('DisplayName')
                if($dn -ne $null -and $dn.Contains('Gravio Sensor Map')) {
                  $ver = $obj.GetValue('DisplayVersion')
                  Write-Host $dn@$ver
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
                    let name = "Sensor Map".to_owned();
                    let version = hubkit_splits[1].trim().to_owned();
                    let identifier = hubkit_splits[0].trim().to_owned();

                    let c = Candidate {
                        name,
                        version,
                        identifier,
                        description: None,
                        installed: true,
                    };
                    installed.push(c);
                }
            } else {
                // Print the error message if the command failed
                eprintln!("PowerShell command failed:\n{:?}", output.status);
                return Err(Box::new(MyError::new("Failed to get installations: GSM")));
            }
        }

        Ok(installed)
    }

    /// Formats a list of Gravio Candidate items into a table and prints to stdout
    pub fn format_candidate_table(&self, candidates: &Vec<Candidate>) {
        log::debug!(
            "Formatting candidate list with {} candidates",
            candidates.len()
        );
        let mut max_len_name: usize = "Candidate".len() + 3;
        let mut max_len_version: usize = "Version".len() + 3;
        let mut max_len_identifier: usize = "Identifier".len() + 5;

        for candidate in candidates {
            if candidate.name.len() > max_len_name {
                max_len_name = candidate.name.len() + 3;
            }
            if candidate.version.len() > max_len_version {
                max_len_version = candidate.version.len() + 3;
            }
            if candidate.identifier.len() > max_len_identifier {
                max_len_identifier = candidate.identifier.len() + 3;
            }
        }

        println!(
            "Candidate | Version | Identifier
----------|---------|-----------"
        );

        for candidate in candidates {
            let cname = format!(
                "{}{:width$}",
                candidate.name,
                " ",
                width = (max_len_name - candidate.name.len())
            );
            let vname = format!(
                "{}{:width$}",
                candidate.version,
                " ",
                width = (max_len_version - candidate.version.len())
            );
            let iname = format!(
                "{}{:width$}",
                candidate.identifier,
                " ",
                width = (max_len_identifier - candidate.identifier.len())
            );
            println!("{}{}{}", cname, vname, iname)
        }
    }
}

#[derive(Debug)]
struct Candidate {
    name: String,
    description: Option<String>,
    version: String,
    identifier: String,
    installed: bool,
}

struct Product {
    name: String,
}

struct ClientConfig {
    repository_folder: Option<String>,
    repository_server: Option<String>,
    repository_credentials: Option<String>,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            repository_server: Some("localhost".to_owned()),
            repository_folder: Some("./".to_owned()),
            repository_credentials: None,
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists installation candidates
    List,
    /// Uninstalls the candidate
    Uninstall { name: String },
    /// Installs the [candidate] with optional [version]
    Install { name: String, ver: Option<String> },
    /// Lists items that are installed on this machine
    Installed,
}

#[derive(Debug)]
struct MyError {
    details: String,
}

impl MyError {
    fn new(msg: &str) -> MyError {
        MyError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MyError {
    fn description(&self) -> &str {
        &self.details
    }
}
#[cfg(test)]
mod tests {

    #[test]
    fn candidates() {
        // list_candidates(None, None);
    }
}
