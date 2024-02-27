use std::str::FromStr;

use lazy_static::lazy_static;
use serde::Deserialize;

use crate::{gman_error::GravioError, platform::Platform};

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
                let result = PackageType::from_str(&val).map_err(|x| {
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
    type Err = GravioError;

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
            _ => Err(GravioError::new("Not a valid PackageType string")),
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Flavor {
    pub platform: Platform,
    pub name: &'static str,
    pub teamcity_id: &'static str,
    pub teamcity_executable_path: &'static str,
    pub package_type: PackageType,
}

impl Flavor {
    pub fn empty() -> Self {
        Self {
            platform: Platform::platform_for_current_platform().unwrap(),
            name: "--",
            teamcity_id: "--",
            teamcity_executable_path: "--",
            package_type: PackageType::Msi,
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
                teamcity_id: "Gravio_GravioStudio4forWindows",
                teamcity_executable_path: "graviostudio.zip",
                package_type: PackageType::AppX,
            },
            Flavor {
                platform: Platform::Windows,
                name: "Sideloading",
                teamcity_id: "Gravio_GravioStudio4forWindows",
                teamcity_executable_path: "graviostudio_sideloading.zip",
                package_type: PackageType::AppX,
            },
            Flavor {
                platform: Platform::Mac,
                name: "DeveloperId",
                teamcity_id: "Gravio_GravioStudio4ForMac",
                teamcity_executable_path: "developerid/GravioStudio.dmg",
                package_type: PackageType::Dmg,

            },
            Flavor {
                platform: Platform::Mac,
                name: "AppStore",
                teamcity_id: "Gravio_GravioStudio4ForMac",
                teamcity_executable_path: "appstore/Gravio Studio.pkg",
                package_type: PackageType::Pkg,
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
                teamcity_id: "Gravio_GravioMonitor",
                name: "GoogleAppStore",
                teamcity_executable_path: "",
                package_type:PackageType::Apk,
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
                teamcity_executable_path: "UpdateManager/build/win/ConfigurationManager.exe",
                teamcity_id: "Gravio_UpdateManager",
                package_type: PackageType::StandaloneExe,
            },
            Flavor{
                platform: Platform::Mac,
                name: "MacUpdateManagerDmg",
                teamcity_executable_path: "UpdateManager/build/macOS/ConfigurationManager",
                teamcity_id: "Gravio_UpdateManager4",
                package_type: PackageType::Dmg,
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
                teamcity_id: "Gravio_GravioHubKit4",
                teamcity_executable_path: "GravioHubKit.msi",
                package_type: PackageType::Msi,
            },
            Flavor{
                platform: Platform::Mac,
                name: "MacHubkit",
                teamcity_id: "Gravio_UpdateManager4",
                teamcity_executable_path: "GravioHubKit.dmg",
                package_type: PackageType::Dmg,

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
                teamcity_id: "Hubble_HubbleForWindows10",
                teamcity_executable_path: "handbookx.msix",
                package_type: PackageType::MsiX,
            },
            Flavor {
                platform: Platform::Windows,
                name: "Sideloading",
                teamcity_id: "Hubble_HubbleForWindows10",
                teamcity_executable_path: "sideloadinghandbookx.msix",
                package_type: PackageType::MsiX,
            },
            Flavor {
                platform: Platform::Android,
                name: "Android",
                teamcity_id: "Hubble_2_HubbleFlutter",
                teamcity_executable_path: "handbookx-release.apk",
                package_type: PackageType::Apk,

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
