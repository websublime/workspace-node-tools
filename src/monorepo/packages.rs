#![warn(dead_code)]
#![allow(clippy::needless_borrow)]

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub private: bool,
    pub package_json_path: String,
    pub package_path: String,
    pub pkg_json: String,
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

                        PackageInfo {
                            name: info.name.clone(),
                            private: info.private,
                            package_json_path,
                            package_path: info.path.clone(),
                            pkg_json: pkg_json.to_string(),
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

                    if patterns.is_match(CandidatePath::from(
                        entry.path().strip_prefix(path).unwrap(),
                    )) {
                        let package_json = std::fs::read_to_string(entry.path()).unwrap();
                        let pkg_json = PackageJson::try_from(package_json).unwrap();
                        let private = match pkg_json.private {
                            Some(package_json_schema::Private::True) => true,
                            _ => false,
                        };
                        let name = pkg_json.name.clone().unwrap();
                        let version = pkg_json.version.clone().unwrap_or(String::from("0.0.0"));
                        let content = pkg_json.to_string();

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
                            pkg_json: content,
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
