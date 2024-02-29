use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use std::fmt::Write;

use reqwest::{
    header::{HeaderValue, RANGE},
    Url,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{
    app,
    candidate::{InstallationCandidate, SearchCandidate, Version},
    gman_error::GManError,
    platform::Platform,
    product::Product,
    CandidateRepository,
};

#[derive(Debug, Deserialize)]
pub struct TeamCityArtifacts {
    #[serde(rename = "count")]
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct TeamCityBuild {
    #[serde(rename = "id")]
    pub id: u32,
    #[serde(rename = "number")]
    pub build_number: String,
    #[serde(rename = "finishDate")]
    pub finish_date: Option<String>,
    #[serde(rename = "artifacts")]
    pub artifacts: Option<TeamCityArtifacts>,
    #[serde(rename = "buildTypeId")]
    pub build_type_id: Option<String>,
    #[serde(rename = "status")]
    pub status: Option<String>,
    #[serde(rename = "branchName")]
    pub branch_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TeamCityBuilds {
    #[serde(rename = "count")]
    pub count: u32,
    #[serde(rename = "build")]
    pub builds: Vec<TeamCityBuild>,
}

#[derive(Debug, Deserialize)]
pub struct TeamCityBranch {
    pub name: String,
    #[serde(deserialize_with = "skip_intermediate_builds_object")]
    pub builds: Vec<TeamCityBuild>,
}

fn skip_intermediate_builds_object<'de, D>(deserializer: D) -> Result<Vec<TeamCityBuild>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    match value {
        Value::Object(kvp) => {
            let mut result = Vec::new();

            let builds = kvp["build"].as_array().unwrap();

            for build_value in builds.to_owned() {
                let build: TeamCityBuild = serde_json::from_value(build_value)
                    .map_err(|e| serde::de::Error::custom(format!("{}", e)))?;
                result.push(build);
            }

            Ok(result)
        }
        _ => Err(serde::de::Error::custom("Expected an array for 'builds'")),
    }
}

#[derive(Debug, Deserialize)]
pub struct TeamCityRoot {
    #[serde(rename = "branch")]
    pub branches: Vec<TeamCityBranch>,
}

/// Ensures that this url starts with 'http://' or 'https://'.
/// If no scheme is provided, 'https://' is pre-pended by default
fn ensure_scheme(url: &str) -> Result<Url, Box<dyn std::error::Error>> {
    let with_scheme = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_owned()
    };
    let u = Url::from_str(&with_scheme)?;
    Ok(u)
}

pub async fn get_builds<'a>(
    http_client: &reqwest::Client,
    current_platform: Platform,
    valid_repositories: &Vec<&CandidateRepository>,
    products: &'a Vec<&Product>,
) -> Result<Vec<InstallationCandidate>, Box<dyn std::error::Error>> {
    let mut candidates: Vec<InstallationCandidate> = Vec::new();

    for repo in valid_repositories {
        if let Some(repo_url) = &repo.repository_server {
            log::debug!(
                "Repo defined a remote url, will fetch from remote '{}'",
                &repo_url
            );

            for product in products {
                log::debug!("Getting builds for {}", &product.name);
                let flavors = product
                    .flavors
                    .iter()
                    .filter(|x| x.platform == current_platform);

                for flavor in flavors {
                    log::debug!("Getting build for flavor {}", &flavor.id);
                    let mut url = ensure_scheme(&repo_url)?;
                    url.set_path(&format!(
                        "app/rest/buildTypes/id:{}/branches",
                        flavor.teamcity_metadata.teamcity_id
                    ));
                    url.query_pairs_mut().append_key_only(
                        "default:true,policy:ACTIVE_HISTORY_AND_ACTIVE_VCS_BRANCHES",
                    );
                    url.set_query(Some("fields=branch(name,builds(build(id,number,finishDate,artifacts($locator(count:1),count:1)),count,$locator(state:finished,status:SUCCESS,count:1)))"));

                    let request: reqwest::Request = match &repo.repository_credentials {
                        Some(credentials) => {
                            let r = http_client.get(url).header("Accept", "Application/json");
                            match credentials {
                                crate::RepositoryCredentials::BearerToken { token } => {
                                    r.bearer_auth(token).build().unwrap()
                                }
                                crate::RepositoryCredentials::BasicAuth { username, password } => {
                                    r.basic_auth(username, password.to_owned()).build().unwrap()
                                }
                            }
                        }
                        None => http_client.get(url).build().unwrap(),
                    };
                    let res = http_client.execute(request).await?;
                    let res_status = res.status();
                    if res_status != 200 {
                        if res_status == 401 || res_status == 403 {
                            eprintln!("Not authorized to access repository {}", &repo.name)
                        } else if res_status == 404 {
                            log::warn!("Repository endpoint not found for repo {}", &repo.name);
                        }
                        log::warn!(
                            "Failed to get TeamCity repository information for repo {}",
                            &repo.name
                        );
                        continue;
                    }

                    let body = res.text().await?;
                    match serde_json::from_str::<TeamCityRoot>(&body) {
                        Ok(team_city_root) => {
                            log::debug!("Got reponse from TeamCity build server");
                            for branch in team_city_root.branches {
                                for build in branch.builds {
                                    let ci = InstallationCandidate {
                                        remote_id: build.id.to_string(),
                                        version: Version::new(build.build_number.as_str()),
                                        identifier: branch.name.to_owned(),
                                        product_name: product.name.to_owned(),
                                        flavor: flavor.to_owned(),
                                        repo_location: repo_url.to_owned(),
                                        installed: false,
                                    };
                                    candidates.push(ci);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to parse TeamCity repository information for repo {}: {}",
                                &repo_url,
                                e,
                            );
                        }
                    }
                }
            }
        } else if let Some(repo_path) = &repo.repository_folder {
            log::debug!("Repo defined a local path, will fetch from file system");
            todo!()
        }
    }

    Ok(candidates)
}

/// Queries TeamCity repositories for the actual internal id of the build given by the [Candidate]
pub async fn get_with_build_id_by_candidate<'a>(
    http_client: &reqwest::Client,
    candidate: &SearchCandidate,
    valid_repositories: &[&'a CandidateRepository],
) -> Result<Option<(InstallationCandidate, &'a CandidateRepository)>, Box<dyn std::error::Error>> {
    if valid_repositories.is_empty() {
        return Err(Box::new(GManError::new(
            "No repositories supplied for searching",
        )));
    }

    for repo in valid_repositories {
        if let Some(repo_url) = &repo.repository_server {
            log::debug!(
                "Repo defined a remote url, will fetch from remote '{}'",
                &repo_url
            );

            let mut url = ensure_scheme(&repo_url)?;
            url.set_path("app/rest/builds");
            let filter_for = if candidate.version.is_some() {
                format!(
                    "number:{}",
                    &<std::option::Option<Version> as Clone>::clone(&candidate.version)
                        .unwrap()
                        .as_ref()
                )
            } else {
                format!("branch:{}", &candidate.identifier.as_ref().unwrap())
            };
            url.query_pairs_mut()
                .append_key_only("default:false,policy:ALL_BRANCHES")
                .append_pair(
                    "locator",
                    &format!(
                        "buildType:{},count:1,{}",
                        &candidate.flavor.teamcity_metadata.teamcity_id, &filter_for
                    ),
                );

            let request: reqwest::Request = match &repo.repository_credentials {
                Some(credentials) => {
                    let r = http_client
                        .get(url.clone())
                        .header("Accept", "Application/json");
                    match credentials {
                        crate::RepositoryCredentials::BearerToken { token } => {
                            r.bearer_auth(token).build().unwrap()
                        }
                        crate::RepositoryCredentials::BasicAuth { username, password } => {
                            r.basic_auth(username, password.to_owned()).build().unwrap()
                        }
                    }
                }
                None => http_client.get(url.clone()).build().unwrap(),
            };

            log::debug!(
                "Sending get_build_id request to repo: {}",
                &url.clone().to_string()
            );

            let res = http_client.execute(request).await?;
            let res_status = res.status();
            if res_status != 200 {
                if res_status == 401 || res_status == 403 {
                    eprintln!("Not authorized to access repository {}", &repo.name)
                } else if res_status == 404 {
                    eprintln!("Repository endpoint not found for repo {}", &repo.name);
                }
                log::warn!(
                    "Failed to get TeamCity repository information for repo {}, status code: {}",
                    &repo.name,
                    res_status
                );
                continue;
            }

            let body = res.text().await?;

            match serde_json::from_str::<TeamCityBuilds>(&body) {
                Ok(team_city_root) => {
                    log::debug!("Got reponse from TeamCity build server");
                    if team_city_root.builds.is_empty() {
                        continue;
                    }
                    for build in team_city_root.builds {
                        let c = InstallationCandidate {
                            remote_id: build.id.to_string(),
                            product_name: candidate.product_name.to_owned(),
                            version: Version::new(build.build_number.as_str()),
                            identifier: build.branch_name.unwrap_or(build.build_number.to_owned()),
                            flavor: candidate.flavor.to_owned(),
                            repo_location: repo_url.to_owned(),
                            installed: false,
                        };
                        return Ok(Some((c, repo)));
                    }
                }
                Err(e) => {
                    log::error!(
                        "Failed to parse TeamCity repository information for repo {} ({})",
                        &repo_url,
                        e,
                    );
                    continue;
                }
            }
        } else if let Some(repo_path) = &repo.repository_folder {
            log::debug!("Repo defined a local path, will fetch from file system");
            todo!()
        }
    }

    Err(Box::new(GManError::new(
        "Unknown error occurred while getting build id: nothing was returned",
    )))
}

/// Downloads the given artifact from the build server, first into the temp directory, and then moves it to the cache directory
pub async fn download_artifact<'a>(
    http_client: &reqwest::Client,
    candidate: &'a InstallationCandidate,
    repo: &CandidateRepository,
    temp_dir: &Path,
    cache_dir: &Path,
    chunk_size: u64,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    log::debug!(
        "Contacting TeamCity for download link on candidate {}",
        &candidate.remote_id
    );

    if let Some(u) = &repo.repository_server {
        let uri_str = format!(
            "{}/repository/download/{}/{}:id/{}",
            u,
            candidate.flavor.teamcity_metadata.teamcity_id,
            candidate.remote_id,
            candidate.flavor.teamcity_metadata.teamcity_binary_path
        );

        let url = ensure_scheme(&uri_str)?;

        log::debug!("Downloading from url {}", &url.as_str());

        /* Send HEAD for file size info */
        let request: reqwest::Request = match &repo.repository_credentials {
            Some(credentials) => {
                let r = http_client.head(url.clone());
                match credentials {
                    crate::RepositoryCredentials::BearerToken { token } => {
                        r.bearer_auth(token).build().unwrap()
                    }
                    crate::RepositoryCredentials::BasicAuth { username, password } => {
                        r.basic_auth(username, password.to_owned()).build().unwrap()
                    }
                }
            }
            None => http_client.get(url.clone()).build().unwrap(),
        };
        let response = http_client.execute(request).await?;
        let res_status = response.status();
        if res_status != 200 {
            log::warn!(
                "Failed to get TeamCity download file size {}, ({})",
                &repo.name,
                &res_status,
            );
            if res_status == 401 || res_status == 403 {
                eprintln!("Not authorized to access repository {}", &repo.name);
                return Err(Box::new(GManError::new("Not authorized")));
            }
            if res_status == 404 {
                eprintln!("File not found on repo {}", &repo.name);
                return Err(Box::new(GManError::new("File not found")));
            }
            return Err(Box::new(GManError::new(
                "Unknown error occurred during download request",
            )));
        }
        let length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .ok_or("response doesn't include the content length")?;
        let length =
            u64::from_str(length.to_str()?).map_err(|_| "invalid Content-Length header")?;

        let output_file_temp_path = &candidate.make_output_for_candidate(temp_dir);
        /* create the parent directory if necessary */
        let prefix = output_file_temp_path.parent().unwrap();
        tokio::fs::create_dir_all(prefix).await?;

        let mut output_file_temp = tokio::fs::File::create(&output_file_temp_path).await?;

        /* Send GET for body */
        let request: reqwest::Request = match &repo.repository_credentials {
            Some(credentials) => {
                let r = http_client.head(url.clone());
                match credentials {
                    crate::RepositoryCredentials::BearerToken { token } => {
                        r.bearer_auth(token).build().unwrap()
                    }
                    crate::RepositoryCredentials::BasicAuth { username, password } => {
                        r.basic_auth(username, password.to_owned()).build().unwrap()
                    }
                }
            }
            None => http_client.get(url.clone()).build().unwrap(),
        };

        let response = http_client.execute(request).await?;
        let res_status = response.status();
        if res_status != 200 {
            log::warn!(
                "Failed to get TeamCity download file size {}, ({})",
                &repo.name,
                &res_status,
            );
            if res_status == 401 || res_status == 403 {
                eprintln!("Not authorized to access repository {}", &repo.name);
                return Err(Box::new(GManError::new("Not authorized")));
            }
            if res_status == 404 {
                eprintln!("File not found on repo {}", &repo.name);
                return Err(Box::new(GManError::new("File not found")));
            }
            return Err(Box::new(GManError::new(
                "Unknown error occurred during download request",
            )));
        }

        /* disable logging here  */
        let last_level = app::disable_logging();
        let progress_bar = ProgressBar::new(length);
        progress_bar.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                .progress_chars("#>-"));

        let mut downloaded: u64 = 0;
        for range in PartialRangeIter::new(0, length - 1, chunk_size)? {
            let request: reqwest::Request = match &repo.repository_credentials {
                Some(credentials) => {
                    let r = http_client.get(url.clone()).header(RANGE, range);
                    match credentials {
                        crate::RepositoryCredentials::BearerToken { token } => {
                            r.bearer_auth(token).build().unwrap()
                        }
                        crate::RepositoryCredentials::BasicAuth { username, password } => {
                            r.basic_auth(username, password.to_owned()).build().unwrap()
                        }
                    }
                }
                None => http_client.get(url.clone()).build().unwrap(),
            };
            let response = http_client.execute(request).await?;

            let status = response.status();
            if !(status == 200 || status == 206) {
                return Err(Box::new(GManError::new("Unexpected error during download")));
            }

            let mut byte_stream = response.bytes_stream();
            while let Some(item) = byte_stream.next().await {
                tokio::io::copy(&mut item?.as_ref(), &mut output_file_temp).await?;
            }

            downloaded += chunk_size;

            progress_bar.set_position(downloaded);
        }

        /* Move file to cache directory */
        let output_file_cache_path = candidate.make_output_for_candidate(cache_dir);
        tokio::fs::rename(&output_file_temp_path, &output_file_cache_path).await?;
        app::enable_logging(last_level);

        Ok(output_file_cache_path)
    } else {
        Err(Box::new(GManError::new(
            "Repository did not have a Server specified",
        )))
    }
}

struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: u64,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: u64) -> Result<Self, Box<dyn std::error::Error>> {
        if buffer_size == 0 {
            Err("invalid buffer_size, give a value greater than zero.")?;
        }
        Ok(PartialRangeIter {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = HeaderValue;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            Some(
                HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1))
                    .expect("string provided by format!"),
            )
        }
    }
}
