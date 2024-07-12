#![warn(dead_code)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::unused_io_amount)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::manager::Agent;
use crate::filesystem::paths::get_project_root_path;
use crate::git::commands::Git;
use execute::Execute;
use package_json_schema::{PackageJson, Repository};
use regex::Regex;
use std::path::Path;
use std::process::{Command, Stdio};
use wax::{CandidatePath, Glob, Pattern};

#[derive(Debug, Deserialize, Serialize)]
struct PnpmInfo {
    name: String,
    path: String,
    private: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct PkgJson {
    workspaces: Vec<String>,
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

pub struct Monorepo;

impl Monorepo {
    /// Get monorepo root path.
    pub fn get_project_root_path() -> Option<String> {
        get_project_root_path()
    }

    /// Get the package manager available in the workspace.
    pub fn get_agent() -> Option<Agent> {
        let path = Monorepo::get_project_root_path().unwrap();
        let path = Path::new(&path);

        Agent::detect(&path)
    }

    /// Get a desription list of packages available in the monorepo
    pub fn get_package_repository_info(url: String) -> PackageRepositoryInfo {
        let regex = Regex::new(r"(?m)((?<protocol>[a-z]+)://)((?<domain>[^/]*)/)(?<org>([^/]*)/)(?<project>(.*))(\.git)?").unwrap();

        let captures = regex.captures(&url).unwrap();
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
    pub fn format_repo_url(repo: Option<Repository>) -> String {
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
                let url = url.unwrap();
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

    /// Validate the minimun config in packages.json files
    pub fn validate_packages_json() -> bool {
        let packages = Monorepo::get_packages();

        for pkg in packages {
            let pkg_json = serde_json::from_value::<PackageJson>(pkg.pkg_json).unwrap();

            let name = pkg_json.name.unwrap_or(String::from("unknown"));
            let version = pkg_json.version.unwrap_or(String::from("0"));
            let description = pkg_json.description.unwrap_or(String::from("unknown"));
            let repository = pkg_json.repository.is_some();
            let files = pkg_json.files.unwrap_or_default();
            let license = pkg_json.license.unwrap_or(String::from("unknown"));

            if name == "unknown" {
                return false;
            }

            if version == "0" {
                return false;
            }

            if description == "unknown" {
                return false;
            }

            if !repository {
                return false;
            }

            if repository {
                let repo = pkg_json.repository.unwrap();
                let repo = match repo {
                    Repository::Path(repo) => repo,
                    Repository::Object { url, .. } => url.unwrap_or(String::from("")),
                };

                if repo.is_empty() {
                    return false;
                }
            }

            if files.is_empty() {
                return false;
            }

            if license == "unknown" {
                return false;
            }
        }

        true
    }

    /// Get a list of packages available in the monorepo
    pub fn get_packages() -> Vec<PackageInfo> {
        return match Monorepo::get_agent() {
            Some(Agent::Pnpm) => {
                let path = Monorepo::get_project_root_path().unwrap();
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
                        let version = pkg_json.version.clone().unwrap_or(String::from("0.0.0"));
                        let is_root = info.path == path;

                        let releative_path = match is_root {
                            true => String::from("."),
                            false => {
                                let mut rel = info.path.strip_prefix(&path).unwrap().to_string();
                                rel.remove(0);
                                rel
                            }
                        };

                        let repo_url = Monorepo::format_repo_url(pkg_json.repository.clone());
                        let repository_info =
                            Monorepo::get_package_repository_info(repo_url.clone());

                        PackageInfo {
                            name: info.name.clone(),
                            private: info.private,
                            package_json_path,
                            package_path: info.path.clone(),
                            package_relative_path: releative_path,
                            pkg_json: serde_json::to_value(&pkg_json).unwrap(),
                            root: is_root,
                            version,
                            url: repo_url,
                            repository_info: Some(repository_info),
                        }
                    })
                    .collect::<Vec<PackageInfo>>()
            }
            Some(Agent::Yarn) | Some(Agent::Npm) => {
                let path = Monorepo::get_project_root_path().unwrap();
                let path = Path::new(&path);
                let package_json = path.join("package.json");
                let mut packages = vec![];

                let package_json = std::fs::read_to_string(package_json).unwrap();

                let PkgJson { mut workspaces, .. } =
                    serde_json::from_str::<PkgJson>(&package_json).unwrap();

                let globs = workspaces
                    .iter_mut()
                    .map(|workspace| {
                        return match workspace.ends_with("/*") {
                            true => {
                                workspace.push_str("*/package.json");
                                Glob::new(workspace).unwrap()
                            }
                            false => {
                                workspace.push_str("/package.json");
                                Glob::new(workspace).unwrap()
                            }
                        };
                    })
                    .collect::<Vec<Glob>>();

                let patterns = wax::any(globs).unwrap();

                let glob = Glob::new("**/package.json").unwrap();

                for entry in glob
                    .walk(path)
                    .not([
                        "**/node_modules/**",
                        "**/src/**",
                        "**/dist/**",
                        "**/tests/**",
                    ])
                    .unwrap()
                {
                    let entry = entry.unwrap();
                    let mut rel_path = entry
                        .path()
                        .strip_prefix(path)
                        .unwrap()
                        .display()
                        .to_string();
                    rel_path.remove(0);

                    if patterns.is_match(CandidatePath::from(
                        entry.path().strip_prefix(path).unwrap(),
                    )) {
                        let package_json = std::fs::read_to_string(entry.path()).unwrap();
                        let pkg_json = PackageJson::try_from(package_json).unwrap();
                        let private =
                            matches!(pkg_json.private, Some(package_json_schema::Private::True));
                        let name = pkg_json.name.clone().unwrap();
                        let version = pkg_json.version.clone().unwrap_or(String::from("0.0.0"));

                        let repo_url = Monorepo::format_repo_url(pkg_json.repository.clone());
                        let repository_info =
                            Monorepo::get_package_repository_info(repo_url.clone());

                        let pkg_info = PackageInfo {
                            name,
                            private,
                            package_json_path: entry.path().to_str().unwrap().to_string(),
                            package_path: entry
                                .path()
                                .parent()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string(),
                            package_relative_path: rel_path,
                            pkg_json: serde_json::to_value(&pkg_json).unwrap(),
                            root: false,
                            version,
                            url: repo_url,
                            repository_info: Some(repository_info),
                        };

                        packages.push(pkg_info);
                    }
                }

                packages
            }
            Some(Agent::Bun) => vec![],
            None => vec![],
        };
    }

    /// Get a list of packages that have changed since a given sha
    pub fn get_changed_packages(sha: Option<String>) -> Vec<PackageInfo> {
        let packages = Monorepo::get_packages();
        let root = Monorepo::get_project_root_path();
        let since = sha.unwrap_or(String::from("main"));

        let changed_files = Git::get_all_files_changed_since_branch(packages.clone(), since, root);

        packages
            .iter()
            .flat_map(|pkg| {
                let mut pkgs = changed_files
                    .iter()
                    .filter(|file| file.starts_with(&pkg.package_path))
                    .map(|_file| PackageInfo {
                        name: pkg.name.clone(),
                        private: pkg.private,
                        package_json_path: pkg.package_json_path.clone(),
                        package_path: pkg.package_path.clone(),
                        package_relative_path: pkg.package_relative_path.clone(),
                        pkg_json: pkg.pkg_json.clone(),
                        root: pkg.root,
                        version: pkg.version.clone(),
                        url: pkg.url.clone(),
                        repository_info: pkg.repository_info.clone(),
                    })
                    .collect::<Vec<PackageInfo>>();

                pkgs.dedup_by(|a, b| a.name == b.name);

                pkgs
            })
            .collect::<Vec<PackageInfo>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{remove_file, File},
        io::Write,
    };

    fn create_agent_file(path: &Path) -> File {
        File::create(path).expect("File not created")
    }

    fn delete_file(path: &Path) {
        remove_file(path).expect("File not deleted");
    }

    fn create_root_package_json(path: &Path) {
        let mut file = File::create(path).expect("File not created");
        file.write(
            r#"
        {
            "name": "@scope/root",
            "version": "0.0.0",
            "workspaces": [
                "packages/package-a",
                "packages/package-b"
            ]
        }"#
            .as_bytes(),
        )
        .expect("File not written");
    }

    fn create_pnpm_workspace(path: &Path) {
        let mut file = File::create(path).expect("File not created");
        file.write(
            r#"
            packages:
                - "packages/*"
        "#
            .as_bytes(),
        )
        .expect("File not written");
    }

    #[test]
    fn monorepo_root_path() {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");

        create_agent_file(&npm_lock);

        let root_path = Monorepo::get_project_root_path();

        assert_eq!(root_path, Some(path.to_str().unwrap().to_string()));

        delete_file(&npm_lock);
    }

    #[test]
    fn monorepo_agent() {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");

        create_agent_file(&npm_lock);

        let agent = Monorepo::get_agent();

        assert_eq!(agent, Some(Agent::Npm));

        delete_file(&npm_lock);
    }

    #[test]
    fn monorepo_npm() {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");
        let package_json = path.join("package.json");

        create_agent_file(&npm_lock);
        create_root_package_json(&package_json);

        let packages = Monorepo::get_packages();

        assert_eq!(packages.len(), 2);

        delete_file(&npm_lock);
        delete_file(&package_json);
    }

    #[test]
    fn monorepo_pnpm() {
        let path = std::env::current_dir().expect("Current user home directory");
        let pnpm_lock = path.join("pnpm-lock.yaml");
        let pnpm_workspace = path.join("pnpm-workspace.yaml");

        create_agent_file(&pnpm_lock);
        create_pnpm_workspace(&pnpm_workspace);

        let packages = Monorepo::get_packages();

        assert_eq!(packages.len(), 2);

        delete_file(&pnpm_lock);
        delete_file(&pnpm_workspace);
    }
}
