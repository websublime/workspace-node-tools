#![allow(clippy::all)]

//! #Packages module
//!
//! The `packages` module is used to get the list of packages available in the monorepo.
use execute::Execute;
use package_json_schema::{PackageJson, Repository};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use wax::{CandidatePath, Glob, Pattern};

use super::git::get_all_files_changed_since_branch;
use super::manager::{detect_package_manager, PackageManager};
use super::paths::get_project_root_path;

#[derive(Debug, Deserialize, Serialize)]
struct PnpmInfo {
    pub name: String,
    pub path: String,
    pub private: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct PkgJson {
    pub workspaces: Vec<String>,
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
    pub changed_files: Vec<String>,
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
    pub changed_files: Vec<String>,
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

impl PackageInfo {
    pub fn push_changed_file(&mut self, file: String) {
        self.changed_files.push(file);
    }

    pub fn get_changed_files(&self) -> Vec<String> {
        self.changed_files.to_vec()
    }

    pub fn extend_changed_files(&mut self, files: Vec<String>) {
        self.changed_files.extend(files);
    }

    pub fn update_version(&mut self, version: String) {
        self.version = version.to_string();
        self.pkg_json["version"] = Value::String(version.to_string());
    }
}

fn get_package_repository_info(url: &String) -> PackageRepositoryInfo {
    let regex = Regex::new(
        r"(?m)((?<protocol>[a-z]+)://)((?<domain>[^/]*)/)(?<org>([^/]*)/)(?<project>(.*))(\.git)?",
    )
    .unwrap();

    let captures = regex.captures(url).unwrap();
    let domain = captures.name("domain").unwrap().as_str();
    let orga = captures.name("org").unwrap().as_str();
    let project = captures.name("project").unwrap().as_str();

    PackageRepositoryInfo {
        domain: domain.to_string().replace("/", ""),
        orga: orga.to_string().replace("/", ""),
        project: project.to_string().replace("/", ""),
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
pub fn get_monorepo_package_manager(cwd: Option<String>) -> Option<PackageManager> {
    let project_root = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let path = Path::new(&project_root);

    detect_package_manager(&path)
}

/// Get a list of packages available in the monorepo
pub fn get_packages(cwd: Option<String>) -> Vec<PackageInfo> {
    let project_root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };
    let package_manager = get_monorepo_package_manager(Some(project_root.to_string()));

    return match package_manager {
        Some(PackageManager::Pnpm) => {
            let path = Path::new(&project_root);
            let pnpm_workspace = path.join("pnpm-workspace.yaml");

            if !pnpm_workspace.as_path().exists() {
                panic!("pnpm-workspace.yaml file not found");
            }

            let mut command = Command::new("pnpm");
            command
                .current_dir(&project_root)
                .arg("list")
                .arg("-r")
                .arg("--depth")
                .arg("-1")
                .arg("--json");

            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let output = command.execute_output().unwrap();
            let pnpm_info =
                serde_json::from_slice::<Vec<PnpmInfo>>(&output.stdout.as_slice()).unwrap();

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
                            let mut rel =
                                info.path.strip_prefix(&project_root).unwrap().to_string();
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
                        changed_files: vec![],
                    }
                })
                .collect::<Vec<PackageInfo>>()
        }
        Some(PackageManager::Yarn) | Some(PackageManager::Npm) => {
            let path = Path::new(&project_root);
            let package_json = path.join("package.json");
            let mut packages = vec![];

            let package_json = std::fs::read_to_string(&package_json).unwrap();

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
                let rel_path = entry
                    .path()
                    .strip_prefix(&path)
                    .unwrap()
                    .display()
                    .to_string();
                //rel_path.remove(0);

                if patterns.is_match(CandidatePath::from(
                    entry.path().strip_prefix(&path).unwrap(),
                )) {
                    let package_json = std::fs::read_to_string(&entry.path()).unwrap();
                    let pkg_json = PackageJson::try_from(package_json).unwrap();
                    let private =
                        matches!(pkg_json.private, Some(package_json_schema::Private::True));

                    let package_json = serde_json::to_value(&pkg_json).unwrap();

                    let repo_url = format_repo_url(&pkg_json.repository);
                    let repository_info = get_package_repository_info(&repo_url);

                    let name = &pkg_json.name.unwrap().to_string();
                    let version = &pkg_json.version.unwrap_or(String::from("0.0.0"));

                    let pkg_info = PackageInfo {
                        name: name.to_string(),
                        private,
                        package_json_path: entry.path().to_str().unwrap().to_string(),
                        package_path: entry.path().parent().unwrap().to_str().unwrap().to_string(),
                        package_relative_path: rel_path
                            .strip_suffix("/package.json")
                            .unwrap()
                            .to_string(),
                        pkg_json: package_json,
                        root: false,
                        version: version.to_string(),
                        url: repo_url,
                        repository_info: Some(repository_info),
                        changed_files: vec![],
                    };

                    packages.push(pkg_info);
                }
            }

            packages
        }
        Some(PackageManager::Bun) => vec![],
        None => vec![],
    };
}

/// Get a list of packages that have changed since a given sha
pub fn get_changed_packages(sha: Option<String>, cwd: Option<String>) -> Vec<PackageInfo> {
    let root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let packages = get_packages(Some(root.to_string()));
    let since = sha.unwrap_or(String::from("main"));

    let changed_files =
        get_all_files_changed_since_branch(&packages, &since, Some(root.to_string()));

    packages
        .iter()
        .flat_map(|pkg| {
            let mut pkgs = changed_files
                .iter()
                .filter(|file| file.starts_with(&pkg.package_path))
                .map(|file| {
                    let mut pkg_info: PackageInfo = pkg.to_owned();
                    pkg_info.changed_files.push(file.to_string());

                    pkg_info
                })
                .collect::<Vec<PackageInfo>>();

            pkgs.dedup_by(|a, b| a.name == b.name);

            pkgs
        })
        .collect::<Vec<PackageInfo>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manager::PackageManager;
    use crate::utils::create_test_monorepo;
    use std::fs::{remove_dir_all, File};
    use std::io::Write;
    use std::path::PathBuf;

    fn create_package_change(monorepo_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let js_path = monorepo_dir.join("packages/package-a/index.js");

        let branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("-b")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git branch problem");

        branch.wait_with_output()?;

        let mut js_file = File::create(&js_path)?;
        js_file
            .write_all(r#"export const message = "hello";"#.as_bytes())
            .unwrap();

        let add = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("add")
            .arg(".")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git add problem");

        add.wait_with_output()?;

        let commit = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("commit")
            .arg("-m")
            .arg("feat: message to the world")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git commit problem");

        commit.wait_with_output()?;

        Ok(())
    }

    #[test]
    fn monorepo_package_manager() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Pnpm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let package_manager = get_monorepo_package_manager(project_root);

        assert_eq!(package_manager, Some(PackageManager::Pnpm));
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn npm_get_packages() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let packages = get_packages(project_root);

        assert_eq!(packages.len(), 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn yarn_get_packages() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Yarn)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let packages = get_packages(project_root);

        assert_eq!(packages.len(), 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn pnpm_get_packages() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Pnpm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let packages = get_packages(project_root);

        assert_eq!(packages.len(), 3);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn monorepo_get_changed_packages() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_package_change(monorepo_dir)?;

        let packages = get_changed_packages(Some("main".to_string()), project_root);

        assert_eq!(packages.len(), 1);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
