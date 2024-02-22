use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub(crate) enum Platform {
    RaspberryPi,
    Linux,
    IOS,
    Windows,
    Mac,
}
