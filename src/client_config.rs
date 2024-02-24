// use log::Log;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;

use crate::platform::Platform;

#[derive(Deserialize, Debug)]
pub(crate) struct CandidateRepository {
    #[serde(rename(deserialize = "Name"))]
    pub name: String,
    #[serde(rename(deserialize = "Platforms"))]
    pub platforms: Vec<Platform>,
    #[serde(rename(deserialize = "RepositoryFolder"))]
    pub repository_folder: Option<String>,
    #[serde(rename(deserialize = "RepositoryServer"))]
    pub repository_server: Option<String>,
    #[serde(rename(deserialize = "RepositoryCredentials"))]
    pub repository_credentials: Option<String>,
}
#[derive(Deserialize, Debug)]
pub(crate) struct ClientConfig {
    /// TeamCity repositories to download artifacts from
    #[serde(rename(deserialize = "Repositories"))]
    pub repositories: Vec<CandidateRepository>,

    /// Location on system to store artifacts while downloading
    ///
    /// Defaults:  
    /// Windows: `%temp%\asteria_gman\`  
    /// Unix: `/tmp/asteria_gman/`
    pub temp_download_directory: Option<String>,

    /// Location where cached downloaded artifacts are stored
    ///
    /// This differs from [temp_download_directory] because only complete artifacts are stored here,
    /// whereas downloads to the temp directory are not guaranteed to be complete (may be in progress, broken, etc)
    pub cache_directory: Option<String>,

    pub log_level: Option<GManLogLevel>,
}

#[derive(Deserialize, Debug)]
pub enum GManLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Off,
}

// impl Into::<log::Level> for GManLogLevel {
//     fn into
// }

impl ClientConfig {
    //     pub fn new() -> Self {
    //         Self {
    //             repositories: Vec::new(),
    //         }
    //     }
}
