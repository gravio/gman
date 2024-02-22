use serde::Deserialize;

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
    #[serde(rename(deserialize = "Repositories"))]
    pub repositories: Vec<CandidateRepository>,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }
}
