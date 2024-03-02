use std::{borrow::Cow, collections::HashMap, env, fs, path::PathBuf, str::FromStr};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{
    app,
    platform::{self, Platform},
    product::{self, Flavor, Product, TeamCityMetadata},
};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct PublisherIdentity {
    /// Display name of this Publisher
    #[serde(rename = "Name")]
    pub name: String,
    /// byte for byte key of this publisher
    #[serde(rename = "Id")]
    pub id: String,
    /// platforms this publisher is used for
    #[serde(rename = "Platforms")]
    pub platforms: Vec<Platform>,
    /// Which product tags this publisher is valid for
    #[serde(rename = "Products")]
    pub products: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "Type")]
pub enum RepositoryCredentials {
    BearerToken {
        #[serde(rename = "Token")]
        token: String,
    },
    BasicAuth {
        #[serde(rename = "Username")]
        username: String,
        #[serde(rename = "Password")]
        password: Option<String>,
    },
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct CandidateRepository {
    /// Display name of this repository
    #[serde(rename = "Name")]
    pub name: String,
    /// Repository type, such as TeamCity
    #[serde(rename = "RepositoryType")]
    pub repository_type: String,

    /// What type of Platform binaries can be found on this repository
    #[serde(rename = "Platforms")]
    pub platforms: Vec<Platform>,

    /// Defines this repository of a local folder
    #[serde(rename = "RepositoryFolder", skip_serializing_if = "Option::is_none")]
    pub repository_folder: Option<String>,

    /// Defines this repository as a remote server
    #[serde(rename = "RepositoryServer")]
    pub repository_server: Option<String>,

    /// API Credentials for this repository
    #[serde(rename = "RepositoryCredentials")]
    pub repository_credentials: Option<RepositoryCredentials>,

    /// Which product tags this publisher is valid for
    #[serde(rename = "Products")]
    pub products: Vec<String>,
}
#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ClientConfig {
    /// TeamCity repositories to download artifacts from
    #[serde(rename = "Repositories")]
    pub repositories: Vec<CandidateRepository>,

    /// Location on system to store artifacts while downloading
    ///
    /// Defaults:  
    /// Windows: `%temp%\graviomanager_5a8f853f-d7e7-4a83-aa21-6ed0585b0c40\`  
    /// Unix: `/tmp/graviomanager_5a8f853f-d7e7-4a83-aa21-6ed0585b0c40/`
    #[serde(
        rename = "TempDownloadDirectory",
        deserialize_with = "deserialize_path_buf_download",
        default = "default_download"
    )]
    pub temp_download_directory: PathBuf,

    /// Location where cached downloaded artifacts are stored
    ///
    /// This differs from [temp_download_directory] because only complete artifacts are stored here,
    /// whereas downloads to the temp directory are not guaranteed to be complete (may be in progress, broken, etc)
    #[serde(
        rename = "CacheDirectory",
        deserialize_with = "deserialize_path_buf_cache",
        default = "default_cache"
    )]
    pub cache_directory: PathBuf,

    /// Log level to display when running this application, defaults to OFF
    #[serde(
        rename = "LogLevel",
        default = "default_log_level",
        deserialize_with = "deserialize_log_level",
        serialize_with = "serialize_log_level"
    )]
    pub log_level: log::LevelFilter,

    /// how large should a packet request to team city be (defaults to 1mb)
    #[serde(rename = "TeamCityDownloadChunkSize", default = "default_chunk_size")]
    pub teamcity_download_chunk_size: u64,

    /// Publisher keys to be aware of when searching for uninstallation material on the local machine
    #[serde(rename = "PublisherIdentities", default = "default_empty_publisher")]
    pub publisher_identities: Vec<PublisherIdentity>,

    #[serde(rename = "Products", default = "default_empty_products")]
    pub products: Vec<Product>,
}
impl ClientConfig {
    /// Creates a sample config suitable for outputting into a json file, for demonstration and rebuilding a config purposes
    pub fn make_sample() -> Self {
        Self {
            log_level: log::LevelFilter::Off,
            cache_directory: default_cache(),
            temp_download_directory: default_download(),
            teamcity_download_chunk_size: default_chunk_size(),
            repositories: vec![CandidateRepository {
                name: "SampleRepository".into(),
                repository_type: "TeamCity".into(),
                platforms: vec![Platform::Windows, Platform::Mac],
                products: vec!["SampleProduct".into()],
                repository_server: Some("yourbuildserver.yourcompany.example.com".into()),
                repository_credentials: Some(RepositoryCredentials::BearerToken {
                    token: "your_token".into(),
                }),
                repository_folder: None,
            }],
            products: vec![product::Product {
                name: "SampleProduct".into(),
                flavors: vec![
                    Flavor {
                        autorun: false,
                        id: "UWP".into(),
                        package_type: product::PackageType::AppX,
                        platform: Platform::Windows,
                        teamcity_metadata: TeamCityMetadata {
                            teamcity_binary_path: "path/to/WindowsUWP.zip".into(),
                            teamcity_id: "SomeUwpSample".into(),
                        },
                        metadata: Some(HashMap::from([(
                            "NameRegex".into(),
                            "some.uwp.sampleproduct".into(),
                        )])),
                    },
                    Flavor {
                        autorun: false,
                        id: "MacApp".into(),
                        package_type: product::PackageType::App,
                        platform: Platform::Mac,
                        teamcity_metadata: TeamCityMetadata {
                            teamcity_binary_path: "path/to/MacApp.dmg".into(),
                            teamcity_id: "SomeMacSample".into(),
                        },
                        metadata: Some(HashMap::from([
                            (
                                "CFBundleIdentifier".into(),
                                "com.somecompany.sampleproduct".into(),
                            ),
                            ("CFBundleName".into(), "SampleProduct".into()),
                        ])),
                    },
                ],
            }],
            publisher_identities: vec![PublisherIdentity {
                id: "CN=ab94ddc1-6575-33ed-8832-1a5d98a25117".into(),
                name: "SomeCompany Windows Identifier".into(),
                products: vec!["SomeProduct".into()],
                platforms: vec![platform::Platform::Windows],
            }],
        }
    }
}

pub const fn default_empty_publisher() -> Vec<PublisherIdentity> {
    Vec::new()
}

pub const fn default_empty_products() -> Vec<Product> {
    Vec::new()
}

pub const fn default_chunk_size() -> u64 {
    1024 * 1024
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<log::LevelFilter, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let de_s = Option::<String>::deserialize(deserializer)
        .map(|opt| opt.unwrap_or_else(|| default_log_level().to_string()));
    Ok(match de_s {
        Ok(s) => log::LevelFilter::from_str(&s).unwrap_or(default_log_level()),
        Err(_) => default_log_level(),
    })
}

fn serialize_log_level<S>(value: &log::LevelFilter, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(value.as_str())
}

const fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Off
}

fn deserialize_path_buf_download<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let de_s = Option::<String>::deserialize(deserializer)
        .map(|opt| opt.unwrap_or_else(|| default_download().to_str().unwrap().to_owned()));
    let pb = match de_s {
        Ok(s) => PathBuf::from_str(ClientConfig::shell_expand(s.as_str()).as_str())
            .unwrap_or(default_download()),
        Err(_) => default_download(),
    };

    Ok(pb)
}

fn default_download() -> PathBuf {
    app::get_app_temp_directory()
}

fn deserialize_path_buf_cache<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let de_s = Option::<String>::deserialize(deserializer)
        .map(|opt| opt.unwrap_or_else(|| default_cache().to_str().unwrap().to_owned()));

    let pb = match de_s {
        Ok(s) => PathBuf::from_str(ClientConfig::shell_expand(s.as_str()).as_str())
            .unwrap_or(default_cache()),
        Err(_) => default_cache(),
    };

    Ok(pb)
}

fn default_cache() -> PathBuf {
    let f = format!("~/.cache/{}", app::APP_FOLDER_NAME);
    let expanded = ClientConfig::shell_expand(&f);
    let pb = PathBuf::from_str(&expanded).expect("Failed to expand default cache directory path");
    pb
}

impl ClientConfig {
    /// Expands ~/ to the users home directory (linux,win),
    /// and %var% to the associated item in windows
    fn shell_expand<'a>(s: &'a str) -> String {
        /* normalize separator */
        let s = if cfg!(windows) {
            s.replace(r"/", r"\")
        } else {
            s.replace(r"\", r"/")
        };

        /* expand the string */
        let expanded: Cow<str> = if cfg!(windows) {
            lazy_static! {
                static ref ENV_VAR: regex::Regex =
                    regex::Regex::new("%([[:word:]]*)%").expect("Failed to create Env Var regex");
            }
            let xyz =
                ENV_VAR.replace_all(&s, |captures: &regex::Captures<'_>| match &captures[1] {
                    "" => String::from("%"),
                    varname => env::var(varname).expect("Bad Var Name"),
                });
            xyz
        } else {
            Cow::Borrowed(&s)
        };
        /* tilde expand */
        let xyz = shellexpand::tilde(&expanded);
        xyz.into_owned()
    }

    /// makes the local temp and cache directories exist. Panics if they can't be created
    pub fn ensure_directories(&self) {
        fs::create_dir_all(&self.cache_directory).expect("Couldn't make Cache Dirctory");
        fs::create_dir_all(&self.temp_download_directory).expect("Couldn't make Temp directory");
    }
}

impl ClientConfig {
    //     pub fn new() -> Self {
    //         Self {
    //             repositories: Vec::new(),
    //         }
    //     }
}

#[cfg(test)]
mod test {
    use crate::ClientConfig;

    #[test]
    fn expand_simple() {
        let s = "some/directory/file.txt";
        let expanded = ClientConfig::shell_expand(s);
        if cfg!(windows) {
            assert_eq!(expanded, "some\\directory\\file.txt");
        } else {
            assert_eq!(expanded, "some/direcory/file.txt");
        }
    }

    #[test]
    fn expand_tilde() {
        let s = "~/some/directory/file.txt";
        let expanded = ClientConfig::shell_expand(s);
        assert!(!expanded.starts_with("~/"))
    }

    #[test]
    fn expand_tmp_win() {
        #[cfg(target_os = "windows")]
        {
            let s = "%temp%/file.txt";
            let expanded = ClientConfig::shell_expand(s);
            assert!(!expanded.starts_with("%temp%"))
        }
    }
}
