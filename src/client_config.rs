use std::{borrow::Cow, env, fs, path::PathBuf, str::FromStr};

use lazy_static::lazy_static;
use serde::Deserialize;

use crate::{app, platform::Platform};

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
        rename = "CascheDirectory",
        deserialize_with = "deserialize_path_buf_cache",
        default = "default_cache"
    )]
    pub cache_directory: PathBuf,

    #[serde(
        rename = "LogLevel",
        default = "default_log_level",
        deserialize_with = "deserialize_log_level"
    )]
    pub log_level: log::LevelFilter,
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
    env::temp_dir().join(app::APP_FOLDER_NAME)
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
                    regex::Regex::new("%([[:word:]]*)%").expect("Invalid Regex");
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
