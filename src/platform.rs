use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(crate) enum Platform {
    Android,
    IOS,
    Windows,
    Mac,
    RaspberryPi,
    Linux,
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
