#![warn(dead_code)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::unused_io_amount)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::manager::Agent;
use crate::filesystem::paths::get_project_root_path;
use execute::Execute;
use package_json_schema::PackageJson;
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

#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub private: bool,
    pub package_json_path: String,
    pub package_path: String,
    pub package_relative_path: String,
    pub pkg_json: Value,
    pub root: bool,
    pub version: String,
}

pub struct Monorepo;

impl Monorepo {
    pub fn get_project_root_path() -> Option<String> {
        get_project_root_path()
    }

    pub fn get_agent() -> Option<Agent> {
        let path = Monorepo::get_project_root_path().unwrap();
        let path = Path::new(&path);

        Agent::detect(&path)
    }

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

                        PackageInfo {
                            name: info.name.clone(),
                            private: info.private,
                            package_json_path,
                            package_path: info.path.clone(),
                            package_relative_path: releative_path,
                            pkg_json: serde_json::to_value(&pkg_json).unwrap(),
                            root: is_root,
                            version,
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
