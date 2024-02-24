use std::str::FromStr;

use bytes::Bytes;
use http_body_util::BodyExt as _;
use hyper;
use reqwest::Url;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use tokio::io::AsyncWriteExt as _;

use crate::{
    candidate::InstallationCandidate,
    gman_error::MyError,
    platform::Platform,
    product::{self, Product},
    Candidate, CandidateRepository,
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
                    log::debug!("Getting build for flavor {}", &flavor.name);
                    let mut url = ensure_scheme(&repo_url)?;
                    url.set_path(&format!(
                        "app/rest/buildTypes/id:{}/branches",
                        flavor.teamcity_id
                    ));
                    url.query_pairs_mut().append_key_only(
                        "default:true,policy:ACTIVE_HISTORY_AND_ACTIVE_VCS_BRANCHES",
                    );
                    url.set_query(Some("fields=branch(name,builds(build(id,number,finishDate,artifacts($locator(count:1),count:1)),count,$locator(state:finished,status:SUCCESS,count:1)))"));

                    let request: reqwest::Request = match &repo.repository_credentials {
                        Some(credentials) => http_client
                            .get(url)
                            .header("Accept", "Application/json")
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
                    match serde_json::from_str::<TeamCityRoot>(&body) {
                        Ok(team_city_root) => {
                            log::debug!("Got reponse from TeamCity build server");
                            for branch in team_city_root.branches {
                                for build in branch.builds {
                                    let ci = InstallationCandidate {
                                        remote_id: build.id.to_string(),
                                        version: build.build_number,
                                        identifier: branch.name.to_owned(),
                                        product_name: product.name.to_owned(),
                                        flavor: flavor.to_owned(),
                                    };
                                    candidates.push(ci);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to parse TeamCity repository information for repo {}",
                                &repo_url
                            );
                        }
                    }
                }
            }
        } else if let Some(repo_path) = &repo.repository_folder {
            log::debug!("Repo defined a local path, will fetch from file system");
        }
    }

    Ok(candidates)
}

/// Queries TeamCity repositories for the actual internal id of the build given by the [Candidate]
pub async fn get_build_id_by_candidate<'a>(
    http_client: &reqwest::Client,
    candidate: &Candidate,
    valid_repositories: &[&CandidateRepository],
) -> Result<Option<Candidate>, Box<dyn std::error::Error>> {
    if valid_repositories.is_empty() {
        return Err(Box::new(MyError::new(
            "No repositories supplied for searching",
        )));
    }
    for repo in valid_repositories {
        // if let Some(repo_url) = repo.repository_server.to_owned() {
        //     let mut url = ensure_scheme(&repo_url)?;
        //     url.set_path("app/rest/builds");

        //     let filter_for = if candidate.version.is_empty() {
        //         format!("branch:{}", &candidate.identifier)
        //     } else {
        //         format!("number:{}", &candidate.version)
        //     };
        //     url.query_pairs_mut().append_pair(
        //         "locator",
        //         &format!(
        //             "buildType:{},count:1,{}",
        //             &candidate.product.teamcity_id, &filter_for
        //         ),
        //     );
        //     let request: reqwest::Request = match &repo.repository_credentials {
        //         Some(credentials) => http_client
        //             .get(url)
        //             .header("Accept", "Application/json")
        //             .bearer_auth(credentials)
        //             .build()
        //             .unwrap(),
        //         None => http_client.get(url).build().unwrap(),
        //     };
        //     let res = http_client.execute(request).await?;
        //     let res_status = res.status();
        //     if res_status != 200 {
        //         if res_status == 401 || res_status == 403 {
        //             eprintln!("Not authorized to access repository {}", &repo.name)
        //         } else if res_status == 404 {
        //             eprintln!("Repository endpoint not found for repo {}", &repo.name);
        //         }
        //         log::warn!(
        //             "Failed to get TeamCity repository information for repo {}",
        //             &repo.name
        //         );
        //         continue;
        //     }
        //     let body = res.text().await?;
        //     match serde_json::from_str::<TeamCityBuilds>(&body) {
        //         Ok(team_city_root) => {
        //             log::debug!("Got reponse from TeamCity build server");
        //             if team_city_root.builds.is_empty() {
        //                 return Ok(None);
        //             }
        //             for build in team_city_root.builds {
        //                 let p: &product::PRODUCT_GRAVIO_STUDIO = &product::PRODUCT_GRAVIO_STUDIO;
        //                 let c = Candidate {
        //                     remote_id: Some(build.id.to_string()),
        //                     description: Some(format!("{} (TeamCity)", &candidate.product.name)),
        //                     installed: false,
        //                     name: p.name.to_owned(),
        //                     version: build.build_number.to_owned(),
        //                     identifier: build.branch_name.unwrap_or(build.build_number.to_owned()),
        //                     product: p,
        //                 };
        //                 return Ok(Some(c));
        //             }
        //         }
        //         Err(e) => {
        //             log::error!(
        //                 "Failed to parse TeamCity repository information for repo {}",
        //                 &repo_url
        //             );
        //             return Err(Box::new(e));
        //         }
        //     }
        // }
    }

    Err(Box::new(MyError::new(
        "Unknown error occurred while getting build id: nothing was returned",
    )))
}

pub async fn download_artifact<'a>(
    candidate: &'a Candidate,
) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!(
        "Contacting TeamCity for download link on candidate {}",
        &candidate.remote_id.as_ref().unwrap()
    );

    // // hyper::
    // // let client = Client::new();

    // http_client
    //     // Fetch the url...
    //     .get(url)
    //     .send()
    //     // And then, if we get a response back...
    //     .and_then(|res| {
    //         println!("Response: {}", res.status());
    //         println!("Headers: {:#?}", res.headers());

    //         let mut file = std::fs::File::create(file_name).unwrap();
    //         // The body is a stream, and for_each returns a new Future
    //         // when the stream is finished, and calls the closure on
    //         // each chunk of the body...
    //         res.into_body().for_each(move |chunk| {
    //             file.write_all(&chunk)
    //                 .map_err(|e| panic!("example expects stdout is open, error={}", e))
    //         })
    //     })
    //     // If all good, just tell the user...
    //     .map(|_| {
    //         println!("\n\nDone.");
    //     })
    //     // If there was an error, let the user know...
    //     .map_err(|err| {
    //         eprintln!("Error {}", err);
    //     });
    Ok(())
}

pub async fn download2(
    fully_qualified_candidate: &Candidate,
    repo: &CandidateRepository,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(u) = &repo.repository_server {
        // let uri_str = format!(
        //     "{}/repository/download/{}/",
        //     u, fully_qualified_candidate.product.teamcity_id
        // );
        // let url = hyper::Uri::from_static("https://google.com");
        // let host = url.host().expect("uri has no host");
        // let port = url.port_u16().unwrap_or(443);
        // let addr = format!("{}:{}", host, port);
        // let stream = tokio::net::TcpStream::connect(addr).await?;
        // let io = hyper_util::rt::TokioIo::new(stream);

        // let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        // tokio::task::spawn(async move {
        //     if let Err(err) = conn.await {
        //         println!("Connection failed: {:?}", err);
        //     }
        // });

        // let authority = url.authority().unwrap().clone();

        // let path = url.path();
        // let req = hyper::Request::builder()
        //     .uri(path)
        //     .header(hyper::header::HOST, authority.as_str())
        //     .body(http_body_util::Empty::<Bytes>::new())?;

        // let mut res = sender.send_request(req).await?;

        // println!("Response: {}", res.status());
        // println!("Headers: {:#?}\n", res.headers());

        // // Stream the body, writing each chunk to stdout as we get it
        // // (instead of buffering and printing at the end).
        // while let Some(next) = res.frame().await {
        //     let frame = next?;
        //     if let Some(chunk) = frame.data_ref() {
        //         tokio::io::stdout().write_all(&chunk).await?;
        //     }
        // }

        // println!("\n\nDone!");
        Ok(())
    } else {
        Err(Box::new(MyError::new(
            "Repository did not have a Server specified",
        )))
    }
}
