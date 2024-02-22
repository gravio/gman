use std::str::FromStr;

use reqwest::Url;

use serde::Deserialize;

use crate::{platform::Platform, product, Candidate, CandidateRepository};

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
    pub finish_date: String,
    #[serde(rename = "artifacts")]
    pub artifacts: TeamCityArtifacts,
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
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "builds")]
    pub builds: Vec<TeamCityBuilds>,
}

#[derive(Debug, Deserialize)]
pub struct TeamCityRoot {
    #[serde(rename = "branch")]
    pub branches: Vec<TeamCityBranch>,
}

pub async fn get_hubkit_builds<'a>(
    http_client: &reqwest::Client,
    valid_repositories: &Vec<&CandidateRepository>,
) -> Result<Vec<Candidate<'a>>, Box<dyn std::error::Error>> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for repo in valid_repositories {
        if let Some(repo_url) = repo.repository_server.to_owned() {
            let with_scheme =
                if !repo_url.starts_with("http://") && !repo_url.starts_with("https://") {
                    format!("https://{}", repo_url)
                } else {
                    repo_url
                };

            let mut url = Url::from_str(&with_scheme)?;
            url.set_path("app/rest/buildTypes/id:Gravio_GravioHubKit4/branches");
            url.query_pairs_mut()
                .append_key_only("default:true,policy:ACTIVE_HISTORY_AND_ACTIVE_VCS_BRANCHES");
            url.set_query(Some("fields=branch(name,builds(build(id,number,finishDate,artifacts($locator(count:1),count:1)),count,$locator(state:finished,status:SUCCESS,count:1)))"));
            let request: reqwest::Request = match &repo.repository_credentials {
                Some(credentials) => http_client
                    .get(url)
                    .bearer_auth(credentials)
                    .build()
                    .unwrap(),
                None => http_client.get(url).build().unwrap(),
            };
            let res = http_client.execute(request).await?;
            let res_status = res.status();
            if res_status != 200 {
                if res_status == 401 || res_status == 403 {
                    eprintln!("Not authorized to access repository {}", &repo.name)
                } else if res_status == 404 {
                    eprintln!("Repository endpoint not found for repo {}", &repo.name);
                }
                log::warn!(
                    "Failed to get TeamCity repository information for repo {}",
                    &repo.name
                );
                continue;
            }
            let body = res.text().await?;
            match serde_xml_rs::from_str::<TeamCityRoot>(&body) {
                Ok(team_city_root) => {
                    log::debug!("Got reponse from TeamCity build server");
                    for branch in team_city_root.branches {
                        for builds in branch.builds {
                            for build in builds.builds {
                                let p = &product::PRODUCT_GRAVIO_HUBKIT;
                                let c = Candidate {
                                    remote_id: Some(build.id.to_string()),
                                    description: Some("Gravio HubKit (TeamCity)".to_owned()),
                                    installed: false,
                                    name: p.name.to_owned(),
                                    version: build.build_number,
                                    identifier: branch.name.to_owned(),
                                    product: p,
                                };

                                candidates.push(c);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "Failed to parse TeamCity repository information for repo {}",
                        &with_scheme
                    );
                }
            }
        }
    }

    Ok(candidates)
}

pub async fn get_studio_builds<'a>(
    http_client: &reqwest::Client,
    valid_repositories: &Vec<&CandidateRepository>,
) -> Result<Vec<Candidate<'a>>, Box<dyn std::error::Error>> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for repo in valid_repositories {
        if let Some(repo_url) = repo.repository_server.to_owned() {
            let with_scheme =
                if !repo_url.starts_with("http://") && !repo_url.starts_with("https://") {
                    format!("https://{}", repo_url)
                } else {
                    repo_url
                };

            let mut url = Url::from_str(&with_scheme)?;
            url.set_path("app/rest/buildTypes/id:Gravio_GravioStudio4forWindows/branches");
            url.query_pairs_mut()
                .append_key_only("default:true,policy:ACTIVE_HISTORY_AND_ACTIVE_VCS_BRANCHES");
            url.set_query(Some("fields=branch(name,builds(build(id,number,finishDate,artifacts($locator(count:1),count:1)),count,$locator(state:finished,status:SUCCESS,count:1)))"));
            let request: reqwest::Request = match &repo.repository_credentials {
                Some(credentials) => http_client
                    .get(url)
                    .bearer_auth(credentials)
                    .build()
                    .unwrap(),
                None => http_client.get(url).build().unwrap(),
            };
            let res = http_client.execute(request).await?;
            let res_status = res.status();
            if res_status != 200 {
                if res_status == 401 || res_status == 403 {
                    eprintln!("Not authorized to access repository {}", &repo.name)
                } else if res_status == 404 {
                    eprintln!("Repository endpoint not found for repo {}", &repo.name);
                }
                log::warn!(
                    "Failed to get TeamCity repository information for repo {}",
                    &repo.name
                );
                continue;
            }
            let body = res.text().await?;
            match serde_xml_rs::from_str::<TeamCityRoot>(&body) {
                Ok(team_city_root) => {
                    log::debug!("Got reponse from TeamCity build server");
                    for branch in team_city_root.branches {
                        for builds in branch.builds {
                            for build in builds.builds {
                                let p: &product::Product =
                                    if repo.platforms.contains(&Platform::Windows) {
                                        &product::PRODUCT_GRAVIO_STUDIO_WINDOWS
                                    } else {
                                        &product::PRODUCT_GRAVIO_STUDIO_MAC
                                    };
                                let c = Candidate {
                                    remote_id: Some(build.id.to_string()),
                                    description: Some("Gravio Studio (TeamCity)".to_owned()),
                                    installed: false,
                                    name: p.name.to_owned(),
                                    version: build.build_number,
                                    identifier: branch.name.to_owned(),
                                    product: p,
                                };

                                candidates.push(c);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "Failed to parse TeamCity repository information for repo {}",
                        &with_scheme
                    );
                }
            }
        }
    }

    Ok(candidates)
}

/// Queries TeamCity repositories for the actual internal id of the build given by the [Candidate]
pub async fn get_build_id_by_candidate<'a>(
    http_client: &reqwest::Client,
    candidate: &Candidate<'a>,
    valid_repositories: &Vec<CandidateRepository>,
) -> Result<String, Box<dyn std::error::Error>> {
    for repo in valid_repositories {
        if let Some(repo_url) = repo.repository_server.to_owned() {
            let with_scheme =
                if !repo_url.starts_with("http://") && !repo_url.starts_with("https://") {
                    format!("https://{}", repo_url)
                } else {
                    repo_url
                };

            let mut url = Url::from_str(&with_scheme)?;
            url.set_path("app/rest/builds");
            url.query_pairs_mut().append_pair(
                "locator",
                "buildType:Gravio_GravioHubKit4,number:5.2.0-7015",
            );
            let request: reqwest::Request = match &repo.repository_credentials {
                Some(credentials) => http_client
                    .get(url)
                    .bearer_auth(credentials)
                    .build()
                    .unwrap(),
                None => http_client.get(url).build().unwrap(),
            };
            let res = http_client.execute(request).await?;
            let res_status = res.status();
            if res_status != 200 {
                if res_status == 401 || res_status == 403 {
                    eprintln!("Not authorized to access repository {}", &repo.name)
                } else if res_status == 404 {
                    eprintln!("Repository endpoint not found for repo {}", &repo.name);
                }
                log::warn!(
                    "Failed to get TeamCity repository information for repo {}",
                    &repo.name
                );
                continue;
            }
            let body = res.text().await?;
            match serde_xml_rs::from_str::<TeamCityRoot>(&body) {
                Ok(team_city_root) => {
                    log::debug!("Got reponse from TeamCity build server");
                    for branch in team_city_root.branches {
                        for builds in branch.builds {
                            for build in builds.builds {
                                let p = &product::PRODUCT_GRAVIO_STUDIO_WINDOWS;
                                let c = Candidate {
                                    remote_id: Some(build.id.to_string()),
                                    description: Some("Gravio Studio (TeamCity)".to_owned()),
                                    installed: false,
                                    name: p.name.to_owned(),
                                    version: build.build_number,
                                    identifier: branch.name.to_owned(),
                                    product: p,
                                };

                                // candidates.push(c);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "Failed to parse TeamCity repository information for repo {}",
                        &with_scheme
                    );
                }
            }
        }
    }
    Ok("".to_owned())
}
// pub async fn download_artifact()
