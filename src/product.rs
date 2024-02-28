use std::str::FromStr;

use lazy_static::lazy_static;
use serde::Deserialize;

use crate::{gman_error::GManError, platform::Platform};

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Product {
    pub name: &'static str,
    pub flavors: Vec<Flavor>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PackageType {
    /// Windows UWP style,
    AppX,
    /// Traditional Windows installer
    Msi,
    /// Modern Windows MSI
    MsiX,
    /// Just a direct windows executable file
    StandaloneExe,
    /// Mac installation (image)
    Dmg,
    /// Mac installation (zip)
    Pkg,
    /// Linux Debian package
    Deb,
    /// Android package
    Apk,
    /// iOS app package
    Ipa,
}

impl PackageType {
    pub fn supported_for_platform(&self, platform: &Platform) -> bool {
        match platform {
            Platform::Android => self == &PackageType::Apk,
            Platform::IOS => self == &PackageType::Ipa,
            Platform::Windows => {
                self == &PackageType::Msi
                    || self == &PackageType::MsiX
                    || self == &PackageType::AppX
            }
            Platform::Mac => self == &PackageType::Apk,
            Platform::RaspberryPi => self == &PackageType::Deb,
            Platform::Linux => self == &PackageType::Deb,
        }
    }
}
impl<'de> Deserialize<'de> for PackageType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(val) => {
                let result = PackageType::from_str(&val).map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&val),
                        &"one of {appx, msi, msix, dmg, pkg, deb, apk, ipa, standaloneexe}",
                    )
                })?;
                Ok(result)
            }
            _ => Err(serde::de::Error::custom(
                "Expected string for 'PackageType'",
            )),
        }
    }
}

impl FromStr for PackageType {
    type Err = GManError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "appx" => Ok(Self::AppX),
            "msi" => Ok(Self::Msi),
            "msix" => Ok(Self::MsiX),
            "standaloneexe" => Ok(Self::StandaloneExe),
            "dmg" => Ok(Self::Dmg),
            "pkg" => Ok(Self::Pkg),
            "deb" => Ok(Self::Deb),
            "apk" => Ok(Self::Apk),
            "ioa" => Ok(Self::Ipa),
            _ => Err(GManError::new("Not a valid PackageType string")),
        }
    }
}
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct TeamCityMetadata {
    pub teamcity_id: &'static str,
    pub teamcity_executable_path: &'static str,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Flavor {
    pub platform: Platform,
    pub name: &'static str,
    pub teamcity_metadata: TeamCityMetadata,
    pub package_type: PackageType,
}

impl Flavor {
    pub fn empty() -> Self {
        Self {
            platform: Platform::platform_for_current_platform().unwrap(),
            name: "--",
            package_type: PackageType::Msi,
            teamcity_metadata: TeamCityMetadata {
                teamcity_id: "--",
                teamcity_executable_path: "--",
            },
        }
    }
}

impl Product {
    pub fn from_name<'a>(product_name: &'_ str) -> Option<&'a Self> {
        match product_name.to_lowercase().trim() {
            "graviostudio" => Some(&PRODUCT_GRAVIO_STUDIO),
            "sensormap" => Some(&PRODUCT_GRAVIO_SENSOR_MAP),
            "monitor" => Some(&PRODUCT_GRAVIO_MONITOR),
            "updatemanager" => Some(&PRODUCT_GRAVIO_UPDATE_MANAGER),
            "hubkit" => Some(&PRODUCT_GRAVIO_HUBKIT),
            "handbookx" => Some(&PRODUCT_HANDBOOK_X),
            _ => None,
        }
    }
}

lazy_static! {
    /* Gravio Studio */
     pub static ref PRODUCT_GRAVIO_STUDIO: Product = Product {
        name: "GravioStudio",
        flavors: vec![
            Flavor {
                platform: Platform::Windows,
                name: "WindowsAppStore",
                package_type: PackageType::AppX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4forWindows",
                    teamcity_executable_path: "graviostudio.zip",
                    },

            },
            Flavor {
                platform: Platform::Windows,
                name: "Sideloading",
                package_type: PackageType::AppX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4forWindows",
                    teamcity_executable_path: "graviostudio_sideloading.zip",
                    },

            },
            Flavor {
                platform: Platform::Mac,
                name: "DeveloperId",
                package_type: PackageType::Dmg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4ForMac",
                    teamcity_executable_path: "developerid/GravioStudio.dmg",
                    },


            },
            Flavor {
                platform: Platform::Mac,
                name: "AppStore",
                package_type: PackageType::Pkg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4ForMac",
                    teamcity_executable_path: "appstore/Gravio Studio.pkg",
                    },

            }
        ],
    };

    /* gsm */
     pub static ref PRODUCT_GRAVIO_SENSOR_MAP: Product = Product {
        name: "SensorMap",
        flavors: Vec::new(),
    };


    /* Monitor */
     pub static ref PRODUCT_GRAVIO_MONITOR: Product = Product {
        name: "Monitor",
        flavors: vec![
            Flavor {
                platform: Platform::Android,
                name: "GoogleAppStore",
                package_type:PackageType::Apk,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioMonitor",
                    teamcity_executable_path: "",
                    },

            }
        ],
    };

    /* Update Manager */
     pub static ref PRODUCT_GRAVIO_UPDATE_MANAGER: Product = Product {
        name: "UpdateManager",
        flavors: vec![
            Flavor{
                platform: Platform::Windows,
                name: "WindowsUpdateManagerExe",
                package_type: PackageType::StandaloneExe,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_executable_path: "UpdateManager/build/win/ConfigurationManager.exe",
                    teamcity_id: "Gravio_UpdateManager",
                    },

            },
            Flavor{
                platform: Platform::Mac,
                name: "MacUpdateManagerDmg",
                package_type: PackageType::Dmg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_executable_path: "UpdateManager/build/macOS/ConfigurationManager",
                    teamcity_id: "Gravio_UpdateManager4",
                    },

            }
        ]
    };
    /* HubKit */
     pub static ref PRODUCT_GRAVIO_HUBKIT: Product = Product {
        name: "HubKit",
        flavors: vec![
            Flavor{
                platform: Platform::Windows,
                name: "WindowsHubkit",
                package_type: PackageType::Msi,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioHubKit4",
                    teamcity_executable_path: "GravioHubKit.msi",
                    },
            },
            Flavor{
                platform: Platform::Mac,
                name: "MacHubkit",
                package_type: PackageType::Dmg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_UpdateManager4",
                    teamcity_executable_path: "GravioHubKit.dmg",
                    },

            },
            // TODO(nf): Linux binaries are named for their version number (i.e., hubkit_5.2.1-8219_all.deb), this makes it hard to automatically extract their binary
        ],
    };

    pub static ref PRODUCT_HANDBOOK_X: Product = Product {
        name: "HandbookX",
        flavors: vec![
            Flavor {
                platform: Platform::Windows,
                name: "Windows",
                package_type: PackageType::MsiX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Hubble_HubbleForWindows10",
                    teamcity_executable_path: "handbookx.msix",
                    },
            },
            Flavor {
                platform: Platform::Windows,
                name: "Sideloading",
                package_type: PackageType::MsiX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Hubble_HubbleForWindows10",
                    teamcity_executable_path: "sideloadinghandbookx.msix",
                    },
            },
            Flavor {
                platform: Platform::Android,
                name: "Android",
                package_type: PackageType::Apk,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Hubble_2_HubbleFlutter",
                    teamcity_executable_path: "handbookx-release.apk",
                    },
            },
        ],
    };

    pub static ref ALL_PRODUCTS: Vec<&'static Product> = vec![
        &PRODUCT_GRAVIO_HUBKIT,
        &PRODUCT_GRAVIO_MONITOR,
        &PRODUCT_GRAVIO_STUDIO,
        &PRODUCT_GRAVIO_SENSOR_MAP,
        &PRODUCT_GRAVIO_UPDATE_MANAGER,
        &PRODUCT_HANDBOOK_X,
    ];
}
