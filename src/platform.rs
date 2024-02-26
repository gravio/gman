use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use crate::gman_error::GravioError;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(crate) enum Platform {
    Android,
    IOS,
    Windows,
    Mac,
    RaspberryPi,
    Linux,
}

impl FromStr for Platform {
    type Err = GravioError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Android" => Ok(Self::Android),
            "iOS" => Ok(Self::IOS),
            "Windows" => Ok(Self::Windows),
            "macOS" => Ok(Self::Mac),
            "rpi" => Ok(Self::RaspberryPi),
            "Linux" => Ok(Self::Linux),
            _ => Err(GravioError::new("Not a valid Platform string")),
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Platform::Android => "Android",
            Platform::IOS => "iOS",
            Platform::Windows => "Windows",
            Platform::Mac => "macOS",
            Platform::RaspberryPi => "rpi",
            Platform::Linux => "Linux",
        })
    }
}

impl Platform {
    /// If this binary is executing on windows, returns Windows; if Mac, returns Mac; otherwise, returns [None]
    pub fn platform_for_current_platform() -> Option<Self> {
        if cfg!(windows) {
            Some(Platform::Windows)
        } else if cfg!(macos) {
            Some(Platform::Mac)
        } else if cfg!(linux) {
            Some(Platform::Linux)
        } else {
            None
        }
    }
}
