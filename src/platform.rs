use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::gman_error::GManError;

#[derive(Debug, PartialEq, Clone, Serialize)]
pub(crate) enum Platform {
    Android,
    IOS,
    Windows,
    Mac,
    RaspberryPi,
    Linux,
}

impl<'de> Deserialize<'de> for Platform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(val) => {
                let result = Platform::from_str(&val).map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&val),
                        &"one of {windows, macos, rpi, linux, android, ios}",
                    )
                })?;
                Ok(result)
            }
            _ => Err(serde::de::Error::custom("Expected string for 'Platform'")),
        }
    }
}

impl FromStr for Platform {
    type Err = GManError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "android" => Ok(Self::Android),
            "ios" => Ok(Self::IOS),
            "windows" => Ok(Self::Windows),
            "mac" | "macos" => Ok(Self::Mac),
            "rpi" => Ok(Self::RaspberryPi),
            "linux" => Ok(Self::Linux),
            _ => Err(GManError::new("Not a valid Platform string")),
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
        #[cfg(target_os = "windows")]
        {
            Some(Platform::Windows)
        }
        #[cfg(target_os = "macos")]
        {
            Some(Platform::Mac)
        }
        #[cfg(target_os = "linux")]
        {
            Some(Platform::Linux)
        }
        #[cfg(target_os = "android")]
        {
            Some(Platform::Android)
        }
        #[cfg(target_os = "ios")]
        {
            Some(Platform::IOS)
        }
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "android",
            target_os = "ios"
        )))]
        None
    }
}
