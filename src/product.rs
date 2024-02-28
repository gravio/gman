use std::str::FromStr;

use lazy_static::lazy_static;
use serde::Deserialize;

use crate::{gman_error::GManError, platform::Platform};

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Product {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Flavors")]
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
    #[serde(rename = "TeamCityId")]
    pub teamcity_id: String,
    #[serde(rename = "TeamCityBinaryPath")]
    pub teamcity_binary_path: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Flavor {
    #[serde(rename = "Platform")]
    pub platform: Platform,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "TeamCityMetadata")]
    pub teamcity_metadata: TeamCityMetadata,
    #[serde(rename = "PackageType")]
    pub package_type: PackageType,
}

impl Flavor {
    pub fn empty() -> Self {
        Self {
            platform: Platform::platform_for_current_platform().unwrap(),
            id: "--".to_owned(),
            package_type: PackageType::Msi,
            teamcity_metadata: TeamCityMetadata {
                teamcity_id: "--".to_owned(),
                teamcity_binary_path: "--".to_owned(),
            },
        }
    }
}

impl Product {
    pub fn from_name<'a>(product_name: &'_ str, products: &Vec<Product>) -> Option<&'a Self> {
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
        name: "GravioStudio".to_owned(),
        flavors: vec![
            Flavor {
                platform: Platform::Windows,
                id: "WindowsAppStore".to_owned(),
                package_type: PackageType::AppX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4forWindows".to_owned(),
                    teamcity_binary_path: "graviostudio.zip".to_owned(),
                    },

            },
            Flavor {
                platform: Platform::Windows,
                id: "Sideloading".to_owned(),
                package_type: PackageType::AppX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4forWindows".to_owned(),
                    teamcity_binary_path: "graviostudio_sideloading.zip".to_owned(),
                    },

            },
            Flavor {
                platform: Platform::Mac,
                id: "DeveloperId".to_owned(),
                package_type: PackageType::Dmg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4ForMac".to_owned(),
                    teamcity_binary_path: "developerid/GravioStudio.dmg".to_owned(),
                    },


            },
            Flavor {
                platform: Platform::Mac,
                id: "MacAppStore".to_owned(),
                package_type: PackageType::Pkg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioStudio4ForMac".to_owned(),
                    teamcity_binary_path: "appstore/Gravio Studio.pkg".to_owned(),
                    },

            }
        ],
    };

    /* gsm */
     pub static ref PRODUCT_GRAVIO_SENSOR_MAP: Product = Product {
        name: "SensorMap".to_owned(),
        flavors: Vec::new(),
    };


    /* Monitor */
     pub static ref PRODUCT_GRAVIO_MONITOR: Product = Product {
        name: "Monitor".to_owned(),
        flavors: vec![
            Flavor {
                platform: Platform::Android,
                id: "GoogleAppStore".to_owned(),
                package_type:PackageType::Apk,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioMonitor".to_owned(),
                    teamcity_binary_path: "".to_owned(),
                    },

            }
        ],
    };

    /* Update Manager */
     pub static ref PRODUCT_GRAVIO_UPDATE_MANAGER: Product = Product {
        name: "UpdateManager".to_owned(),
        flavors: vec![
            Flavor{
                platform: Platform::Windows,
                id: "WindowsUpdateManagerExe".to_owned(),
                package_type: PackageType::StandaloneExe,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_binary_path: "UpdateManager/build/win/ConfigurationManager.exe".to_owned(),
                    teamcity_id: "Gravio_UpdateManager".to_owned(),
                    },

            },
            Flavor{
                platform: Platform::Mac,
                id: "MacUpdateManagerDmg".to_owned(),
                package_type: PackageType::Dmg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_binary_path: "UpdateManager/build/macOS/ConfigurationManager".to_owned(),
                    teamcity_id: "Gravio_UpdateManager4".to_owned(),
                    },

            }
        ]
    };
    /* HubKit */
     pub static ref PRODUCT_GRAVIO_HUBKIT: Product = Product {
        name: "HubKit".to_owned(),
        flavors: vec![
            Flavor{
                platform: Platform::Windows,
                id: "WindowsHubkit".to_owned(),
                package_type: PackageType::Msi,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioHubKit4".to_owned(),
                    teamcity_binary_path: "GravioHubKit.msi".to_owned(),
                    },
            },
            Flavor{
                platform: Platform::Mac,
                id: "MacHubkit".to_owned(),
                package_type: PackageType::Dmg,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Gravio_GravioHubKit4".to_owned(),
                    teamcity_binary_path: "GravioHubKit.dmg".to_owned(),
                    },

            },
            // TODO(nf): Linux binaries are named for their version number (i.e., hubkit_5.2.1-8219_all.deb), this makes it hard to automatically extract their binary
        ],
    };

    pub static ref PRODUCT_HANDBOOK_X: Product = Product {
        name: "HandbookX".to_owned(),
        flavors: vec![
            Flavor {
                platform: Platform::Windows,
                id: "Windows".to_owned(),
                package_type: PackageType::MsiX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Hubble_HubbleForWindows10".to_owned(),
                    teamcity_binary_path: "handbookx.msix".to_owned(),
                    },
            },
            Flavor {
                platform: Platform::Windows,
                id: "Sideloading".to_owned(),
                package_type: PackageType::MsiX,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Hubble_HubbleForWindows10".to_owned(),
                    teamcity_binary_path: "sideloadinghandbookx.msix".to_owned(),
                    },
            },
            Flavor {
                platform: Platform::Android,
                id: "Android".to_owned(),
                package_type: PackageType::Apk,
                teamcity_metadata: TeamCityMetadata {
                    teamcity_id: "Hubble_2_HubbleFlutter".to_owned(),
                    teamcity_binary_path: "handbookx-release.apk".to_owned(),
                    },
            },
        ],
    };


}
