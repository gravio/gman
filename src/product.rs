use lazy_static::lazy_static;

use crate::platform::Platform;

#[derive(Debug, PartialEq, Clone)]
pub struct Product {
    pub name: &'static str,
    pub flavors: Vec<Flavor>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Flavor {
    pub platform: Platform,
    pub name: &'static str,
    pub teamcity_id: &'static str,
    pub teamcity_executable_path: &'static str,
}

impl Flavor {
    pub fn empty() -> Self {
        Self {
            platform: Platform::platform_for_current_platform().unwrap(),
            name: "--",
            teamcity_id: "--",
            teamcity_executable_path: "--",
        }
    }
}

impl Product {
    pub fn from_name<'a>(product_name: &'_ str) -> Option<&'a Self> {
        match product_name.to_lowercase().trim() {
            "graviostudio" => Some(&PRODUCT_GRAVIO_STUDIO),
            "sensormap" => Some(&PRODUCT_GRAVIO_SENSOR_MAP),
            "monitor" => Some(&PRODUCT_GRAVIO_MONITOR),
            "updatemanager" => Some(&PRODUCT_UPDATE_MANAGER),
            "hubkit" => Some(&PRODUCT_GRAVIO_HUBKIT),
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
                teamcity_executable_path: "graviostudio.zip"
            },
            Flavor {
                platform: Platform::Windows,
                name: "Sideloading",
                teamcity_id: "Gravio_GravioStudio4forWindows",
                teamcity_executable_path: "graviostudio_sideloading.zip"
            },

            Flavor {
                platform: Platform::Mac,
                name: "DeveloperId",
                teamcity_id: "Gravio_GravioStudio4ForMac",
                teamcity_executable_path: "developerid/GravioStudio.dmg"
            },
            Flavor {
                platform: Platform::Mac,
                name: "AppStore",
                teamcity_id: "Gravio_GravioStudio4ForMac",
                teamcity_executable_path: "appstore/Gravio Studio.pkg"
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
            }
        ],
    };

    /* Update Manager */
     pub static ref PRODUCT_UPDATE_MANAGER: Product = Product {
        name: "UpdateManager",
        flavors: vec![
            Flavor{
                platform: Platform::Windows,
                name: "WindowsUpdateManagerExe",
                teamcity_executable_path: "UpdateManager/build/win/ConfigurationManager.exe",
                teamcity_id: "Gravio_UpdateManager",
            },
            Flavor{
                platform: Platform::Mac,
                name: "MacUpdateManagerDmg",
                teamcity_executable_path: "UpdateManager/build/macOS/ConfigurationManager",
                teamcity_id: "Gravio_UpdateManager4",
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
            },
            Flavor{
                platform: Platform::Mac,
                name: "MacHubkit",
                teamcity_id: "Gravio_UpdateManager4",
                teamcity_executable_path: "GravioHubKit.dmg",
            },
            // TODO(nf): Linux binaries are named for their version number (i.e., hubkit_5.2.1-8219_all.deb), this makes it hard to automatically extract their binary
        ],
    };

    pub static ref ALL_PRODUCTS: Vec<&'static Product> = vec![
        &PRODUCT_GRAVIO_HUBKIT,
        &PRODUCT_GRAVIO_MONITOR,
        &PRODUCT_GRAVIO_STUDIO,
        &PRODUCT_GRAVIO_SENSOR_MAP,
        &PRODUCT_UPDATE_MANAGER,
    ];
}
