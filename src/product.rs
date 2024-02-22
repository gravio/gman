use lazy_static::lazy_static;

use crate::platform::Platform;

#[derive(Debug, PartialEq)]
pub struct Product {
    pub name: &'static str,
    pub teamcity_id: &'static str,
}

impl Product {
    pub fn from_name_and_platform(name: &str, platform: Option<Platform>) -> Option<&Self> {
        match name {
            "GravioStudio" => match platform {
                Some(p) => match p {
                    Platform::Windows => Some(&PRODUCT_GRAVIO_STUDIO_WINDOWS),
                    Platform::Mac => Some(&PRODUCT_GRAVIO_STUDIO_MAC),
                    _ => None,
                },
                None => None,
            },
            "SensorMap" => Some(&PRODUCT_GRAVIO_SENSOR_MAP),
            "Monitor" => Some(&PRODUCT_GRAVIO_MONITOR),
            "UpdateManager" => match platform {
                Some(p) => match p {
                    Platform::Windows => Some(&PRODUCT_UPDATE_MANAGER_WINDOWS),
                    _ => Some(&PRODUCT_UPDATE_MANAGER_UNIX),
                },
                None => None,
            },
            "Deploy" => Some(&PRODUCT_GRAVIO_DEPLOY),
            "TrainingCoordinator" => Some(&PRODUCT_GRAVIO_TRAINING_COORDINATOR),
            "gravio.com" => Some(&PRODUCT_GRAVIO_DOT_COM),
            "HubKit" => Some(&PRODUCT_GRAVIO_HUBKIT),
            _ => None,
        }
    }
}

lazy_static! {
    /* gs/win */
     pub static ref PRODUCT_GRAVIO_STUDIO_WINDOWS: Product = Product {
        name: "GravioStudio",
        teamcity_id: "Gravio_GravioStudio4forWindows"
    };

    /* gs/mac */
     pub static ref PRODUCT_GRAVIO_STUDIO_MAC: Product = Product {
        name: "GravioStudio",
        teamcity_id: "Gravio_GravioStudio4ForMac"
    };

    /* gsm */
     pub static ref PRODUCT_GRAVIO_SENSOR_MAP: Product = Product {
        name: "SensorMap",
        teamcity_id: ""
    };


    /* Monitor */
     pub static ref PRODUCT_GRAVIO_MONITOR: Product = Product {
        name: "Monitor",
        teamcity_id: "Gravio_GravioMonitor"
    };

    /* Update Manager /win  */
     pub static ref PRODUCT_UPDATE_MANAGER_WINDOWS: Product = Product {
        name: "UpdateManager",
        teamcity_id: "Gravio_UpdateManager"
    };

    /* Update Manager /linux,mac */
     pub static ref PRODUCT_UPDATE_MANAGER_UNIX: Product = Product {
        name: "UpdateManager",
        teamcity_id: "Gravio_UpdateManager4"
    };

    /* Deploy */
     pub static ref PRODUCT_GRAVIO_DEPLOY: Product = Product {
        name: "Deploy",
        teamcity_id: "Gravio_GravioDeploy"
    };

    /* Training Coordinator */
     pub static ref PRODUCT_GRAVIO_TRAINING_COORDINATOR: Product = Product {
        name: "TrainingCoordinator",
        teamcity_id: "Gravio_GravioTrainingCoordinato"
    };

    /* gravio.com */
     pub static ref PRODUCT_GRAVIO_DOT_COM: Product = Product {
        name: "gravio.com",
        teamcity_id: "Gravio_GraveComGoLangVersion"
    };

    /* HubKit */
     pub static ref PRODUCT_GRAVIO_HUBKIT: Product = Product {
        name: "HubKit",
        teamcity_id: "Gravio_GravioHubKit4"
    };
}
