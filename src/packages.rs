use execute::Execute;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::process::{Command, Stdio};
use package_json_schema::{PackageJson, Repository};
use regex::Regex;

use super::manager::{PackageManager, detect_package_manager};
use super::paths::get_project_root_path;

#[derive(Debug, Deserialize, Serialize)]
struct PnpmInfo {
    name: String,
    path: String,
    private: bool,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PackageInfo {
    pub name: String,
    pub private: bool,
    pub package_json_path: String,
    pub package_path: String,
    pub package_relative_path: String,
    pub pkg_json: Value,
    pub root: bool,
    pub version: String,
    pub url: String,
    pub repository_info: Option<PackageRepositoryInfo>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PackageInfo {
    pub name: String,
    pub private: bool,
    pub package_json_path: String,
    pub package_path: String,
    pub package_relative_path: String,
    pub pkg_json: Value,
    pub root: bool,
    pub version: String,
    pub url: String,
    pub repository_info: Option<PackageRepositoryInfo>,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PackageRepositoryInfo {
    pub domain: String,
    pub orga: String,
    pub project: String,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PackageRepositoryInfo {
    pub domain: String,
    pub orga: String,
    pub project: String,
}

fn get_package_repository_info(url: &String) -> PackageRepositoryInfo {
    let regex = Regex::new(r"(?m)((?<protocol>[a-z]+)://)((?<domain>[^/]*)/)(?<org>([^/]*)/)(?<project>(.*))(\.git)?").unwrap();

    let captures = regex.captures(url).unwrap();
    let domain = captures.name("domain").unwrap().as_str();
    let orga = captures.name("org").unwrap().as_str();
    let project = captures.name("project").unwrap().as_str();

    PackageRepositoryInfo {
        domain: domain.to_string(),
        orga: orga.to_string(),
        project: project.to_string(),
    }
}

/// Generates and format the url of the project
fn format_repo_url(repo: &Option<Repository>) -> String {
    let regex = Regex::new(r"(?m)^((?<prefix>git[/+]))?((?<protocol>https?|ssh|git|ftps?)://)?((?<user>[^/@]+)@)?(?<host>[^/:]+)[/:](?<port>[^/:]+)/(?<path>.+/)?(?<repo>.+?)(?<suffix>\.git[/]?)?$").unwrap();

    match repo {
        Some(Repository::Path(repo)) => {
            let captures = regex.captures(&repo).unwrap();
            let mut url = "https://".to_string();

            if captures.name("host").is_some() {
                url.push_str(captures.name("host").unwrap().as_str());
            }

            if captures.name("port").is_some() {
                url.push('/');
                url.push_str(captures.name("port").unwrap().as_str());
            }

            if captures.name("path").is_some() {
                url.push('/');
                url.push_str(captures.name("repo").unwrap().as_str());
            }

            if captures.name("repo").is_some() {
                url.push('/');
                url.push_str(captures.name("repo").unwrap().as_str());
            }

            url
        }
        Some(Repository::Object { url, .. }) => {
            let url = url.as_ref().unwrap().to_string();
            let captures = regex.captures(&url).unwrap();
            let mut url = "https://".to_string();

            if captures.name("host").is_some() {
                url.push_str(captures.name("host").unwrap().as_str());
            }

            if captures.name("port").is_some() {
                url.push('/');
                url.push_str(captures.name("port").unwrap().as_str());
            }

            if captures.name("path").is_some() {
                url.push('/');
                url.push_str(captures.name("repo").unwrap().as_str());
            }

            if captures.name("repo").is_some() {
                url.push('/');
                url.push_str(captures.name("repo").unwrap().as_str());
            }

            url
        }
        None => String::from("https://github.com/my-orga/my-repo"),
    }
}

/// Get defined package manager in the monorepo
pub fn get_monorepo_package_manager() -> Option<PackageManager> {
    let project_root = get_project_root_path().unwrap();
    let path = Path::new(&project_root);

    detect_package_manager(&path)
}

/// Get a list of packages available in the monorepo
pub fn get_packages() -> Vec<PackageInfo> {
    let package_manager = get_monorepo_package_manager();
    let project_root = get_project_root_path().unwrap();

    return match package_manager {
        Some(PackageManager::Pnpm) => {
            let mut command = Command::new("pnpm");
            command
                .arg("list")
                .arg("-r")
                .arg("--depth")
                .arg("-1")
                .arg("--json");

            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let output = command.execute_output().unwrap();
            let output = String::from_utf8(output.stdout).unwrap();

            let pnpm_info = serde_json::from_str::<Vec<PnpmInfo>>(&output).unwrap();

            pnpm_info
                .iter()
                .map(|info| {
                    let package_json_path = format!("{}/package.json", info.path);
                    let package_json = std::fs::read_to_string(&package_json_path).unwrap();
                    let pkg_json = PackageJson::try_from(package_json).unwrap();
                    let is_root = info.path == project_root;

                    let relative_path = match is_root {
                        true => String::from("."),
                        false => {
                            let mut rel = info.path.strip_prefix(&project_root).unwrap().to_string();
                            rel.remove(0);
                            rel
                        }
                    };

                    let repo_url = format_repo_url(&pkg_json.repository);
                    let repository_info = get_package_repository_info(&repo_url);

                    let name = &info.name.to_string();
                    let package_path = &info.path.to_string();
                    let package_json = serde_json::to_value(&pkg_json).unwrap();
                    let version = &pkg_json.version.unwrap_or(String::from("0.0.0"));

                    PackageInfo {
                        name: name.to_string(),
                        private: info.private,
                        package_json_path,
                        package_path: package_path.to_string(),
                        package_relative_path: relative_path,
                        pkg_json: package_json,
                        root: is_root,
                        version: version.to_string(),
                        url: String::from(repo_url),
                        repository_info: Some(repository_info),
                    }
                })
                .collect::<Vec<PackageInfo>>()
        }
        Some(PackageManager::Yarn) | Some(PackageManager::Npm) => vec![],
        Some(PackageManager::Bun) => vec![],
        None => vec![],
    };
}