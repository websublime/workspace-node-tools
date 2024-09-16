#![allow(clippy::all)]

//! #Packages module
//!
//! The `packages` module is used to get the list of packages available in the monorepo.
use execute::Execute;
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
/// A struct that represents a pnpm workspace.
struct PnpmInfo {
    pub name: String,
    pub path: String,
    pub private: bool,
}

#[derive(Debug, Deserialize, Serialize)]
/// A struct that represents a yarn workspace.
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
/// A struct that represents a package in the monorepo.
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
/// A struct that represents the repository information of a package.
pub struct PackageRepositoryInfo {
    pub domain: String,
    pub orga: String,
    pub project: String,
}

impl PackageInfo {
    /// Pushes a changed file to the list of changed files.
    pub fn push_changed_file(&mut self, file: String) {
        self.changed_files.push(file);
    }

    /// Returns the list of changed files.
    pub fn get_changed_files(&self) -> Vec<String> {
        self.changed_files.to_vec()
    }

    /// Extends the list of changed files with the provided list.
    pub fn extend_changed_files(&mut self, files: Vec<String>) {
        let founded_files = files
            .iter()
            .filter(|file| file.starts_with(&self.package_path))
            .map(|file| file.to_string())
            .collect::<Vec<String>>();

        self.changed_files.extend(founded_files);
    }

    /// Updates the version of the package.
    pub fn update_version(&mut self, version: String) {
        self.version = version.to_string();
        self.pkg_json["version"] = Value::String(version.to_string());
    }

    /// Updates a dependency version in the package.json file.
    pub fn update_dependency_version(&mut self, dependency: String, version: String) {
        let package_json = self.pkg_json.as_object().unwrap();

        if package_json.contains_key("dependencies") {
            let dependencies = self.pkg_json["dependencies"].as_object_mut().unwrap();
            let has_dependency = dependencies.contains_key(&dependency);

            if has_dependency {
                dependencies.insert(dependency, Value::String(version));
            }
        }
    }

    /// Updates a dev dependency version in the package.json file.
    pub fn update_dev_dependency_version(&mut self, dependency: String, version: String) {
        let package_json = self.pkg_json.as_object().unwrap();

        if package_json.contains_key("devDependencies") {
            let dev_dependencies = self.pkg_json["devDependencies"].as_object_mut().unwrap();
            let has_dependency = dev_dependencies.contains_key(&dependency);

            if has_dependency {
                dev_dependencies.insert(dependency, Value::String(version));
            }
        }
    }

    /// Write package.json file with the updated version.
    pub fn write_package_json(&self) {
        let package_json_file = std::fs::File::create(&self.package_json_path).unwrap();
        let package_json_writer = std::io::BufWriter::new(package_json_file);

        serde_json::to_writer_pretty(package_json_writer, &self.pkg_json).unwrap();
    }
}

/// Returns package info domain, scope and repository name.
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
        project: project.to_string().replace("/", "").replace(".git", ""),
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
                    let ref package_json_path = format!("{}/package.json", info.path);

                    let package_json_file =
                        std::fs::File::open(package_json_path.to_string()).unwrap();
                    let package_json_reader = std::io::BufReader::new(package_json_file);
                    let pkg_json: serde_json::Value =
                        serde_json::from_reader(package_json_reader).unwrap();

                    let ref version = match pkg_json.get("version") {
                        Some(version) => {
                            if version.is_string() {
                                version.as_str().unwrap().to_string()
                            } else {
                                String::from("0.0.0")
                            }
                        }
                        None => String::from("0.0.0"),
                    };

                    let ref repo_url = match pkg_json.get("repository") {
                        Some(repository) => {
                            if repository.is_object() {
                                let repo = repository.as_object().unwrap();

                                match repo.get("url") {
                                    Some(url) => url.as_str().unwrap().to_string(),
                                    None => String::from("https://github.com/my-orga/my-repo"),
                                }
                            } else if repository.is_string() {
                                repository.as_str().unwrap().to_string()
                            } else {
                                String::from("https://github.com/my-orga/my-repo")
                            }
                        }
                        None => String::from("https://github.com/my-orga/my-repo"),
                    };

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

                    let repository_info = get_package_repository_info(repo_url);
                    let name = &info.name.to_string();
                    let package_path = &info.path.to_string();

                    PackageInfo {
                        name: name.to_string(),
                        private: info.private,
                        package_json_path: package_json_path.to_string(),
                        package_path: package_path.to_string(),
                        package_relative_path: relative_path,
                        pkg_json,
                        root: is_root,
                        version: version.to_string(),
                        url: String::from(repo_url),
                        repository_info: Some(repository_info),
                        changed_files: vec![],
                    }
                })
                .filter(|pkg| !pkg.root)
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
                    let package_json_file = std::fs::File::open(&entry.path()).unwrap();
                    let package_json_reader = std::io::BufReader::new(package_json_file);
                    let pkg_json: serde_json::Value =
                        serde_json::from_reader(package_json_reader).unwrap();

                    let private = match pkg_json.get("private") {
                        Some(private) => {
                            if private.is_boolean() {
                                private.as_bool().unwrap()
                            } else {
                                false
                            }
                        }
                        None => false,
                    };

                    let ref version = match pkg_json.get("version") {
                        Some(version) => {
                            if version.is_string() {
                                version.as_str().unwrap().to_string()
                            } else {
                                String::from("0.0.0")
                            }
                        }
                        None => String::from("0.0.0"),
                    };

                    let ref repo_url = match pkg_json.get("repository") {
                        Some(repository) => {
                            if repository.is_object() {
                                let repo = repository.as_object().unwrap();

                                match repo.get("url") {
                                    Some(url) => url.as_str().unwrap().to_string(),
                                    None => String::from("https://github.com/my-orga/my-repo"),
                                }
                            } else if repository.is_string() {
                                repository.as_str().unwrap().to_string()
                            } else {
                                String::from("https://github.com/my-orga/my-repo")
                            }
                        }
                        None => String::from("https://github.com/my-orga/my-repo"),
                    };

                    let name = match pkg_json.get("name") {
                        Some(name) => {
                            if name.is_string() {
                                name.as_str().unwrap().to_string()
                            } else {
                                String::from("unknown")
                            }
                        }
                        None => String::from("unknown"),
                    };

                    let repository_info = get_package_repository_info(repo_url);

                    let pkg_info = PackageInfo {
                        name: name.to_string(),
                        private,
                        package_json_path: entry.path().to_str().unwrap().to_string(),
                        package_path: entry.path().parent().unwrap().to_str().unwrap().to_string(),
                        package_relative_path: rel_path
                            .strip_suffix("/package.json")
                            .unwrap()
                            .to_string(),
                        pkg_json,
                        root: false,
                        version: version.to_string(),
                        url: repo_url.to_string(),
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
                    pkg_info.push_changed_file(file.to_string());

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
    use std::process::Command;

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

        assert_eq!(packages.len(), 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn monorepo_get_changed_packages() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_package_change(monorepo_dir)?;

        let packages = get_changed_packages(Some("main".to_string()), project_root);
        let package = packages.first();

        let changed_files = package.unwrap().get_changed_files();

        assert_eq!(packages.len(), 1);
        assert_eq!(changed_files.len(), 1);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
