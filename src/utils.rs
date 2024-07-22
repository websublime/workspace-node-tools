#![allow(clippy::all)]
#![allow(dead_code)]

use regex::Regex;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use std::fs::{create_dir, File};
#[cfg(test)]
use std::io::Write;

#[cfg(test)]
use rand::distributions::Alphanumeric;
#[cfg(test)]
use rand::{thread_rng, Rng};

#[cfg(test)]
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;

#[cfg(test)]
use super::manager::PackageManager;
#[cfg(test)]
use std::process::Command;
#[cfg(test)]
use std::process::Stdio;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Package scope metadata extracted from a package name.
pub struct PackageScopeMetadata {
    pub full: String,
    pub name: String,
    pub version: String,
    pub path: Option<String>,
}

/// Extracts the package scope name and version from a package name.
pub(crate) fn package_scope_name_version(pkg_name: &str) -> Option<PackageScopeMetadata> {
    let regex = Regex::new("^((?:@[^/@]+/)?[^/@]+)(?:@([^/]+))?(/.*)?$").unwrap();

    let matches = regex.captures(pkg_name).unwrap();

    if matches.len() > 0 {
        return Some(PackageScopeMetadata {
            full: matches.get(0).map_or("", |m| m.as_str()).to_string(),
            name: matches.get(1).map_or("", |m| m.as_str()).to_string(),
            version: matches.get(2).map_or("", |m| m.as_str()).to_string(),
            path: matches
                .get(3)
                .map_or(None, |m| Some(m.as_str().to_string())),
        });
    }

    None
}

/// Strips the trailing newline from a string.
pub(crate) fn strip_trailing_newline(input: &String) -> String {
    input
        .strip_suffix("\r\n")
        .or(input.strip_suffix("\n"))
        .unwrap_or(input)
        .trim()
        .to_string()
}

#[cfg(test)]
pub(crate) fn create_test_monorepo(
    package_manager: &PackageManager,
) -> Result<std::path::PathBuf, std::io::Error> {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    let temp_dir = std::env::temp_dir();
    let monorepo_temp_dir = temp_dir.join(format!("monorepo-{}", rand_string));
    let monorepo_package_json = monorepo_temp_dir.join("package.json");

    let monorepo_packages_dir = monorepo_temp_dir.join("packages");
    let monorepo_package_a_dir = monorepo_packages_dir.join("package-a");
    let monorepo_package_b_dir = monorepo_packages_dir.join("package-b");

    create_dir(&monorepo_temp_dir)?;
    create_dir(&monorepo_packages_dir)?;
    create_dir(&monorepo_package_a_dir)?;
    create_dir(&monorepo_package_b_dir)?;

    #[cfg(not(windows))]
    std::fs::set_permissions(&monorepo_temp_dir, std::fs::Permissions::from_mode(0o777))?;

    let mut monorepo_package_json_file = File::create(&monorepo_package_json)?;
    monorepo_package_json_file.write_all(
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
    )?;

    let mut monorepo_package_a_json = File::create(&monorepo_package_a_dir.join("package.json"))?;
    let mut monorepo_package_b_json = File::create(&monorepo_package_b_dir.join("package.json"))?;

    monorepo_package_a_json.write_all(
        r#"
        {
            "name": "@scope/package-a",
              "version": "1.0.0",
              "description": "My new package A",
              "main": "index.mjs",
              "repository": {
                "url": "git+ssh://git@github.com/websublime/workspace-node-binding-tools.git",
                "type": "git"
              },
              "scripts": {
                "test": "echo \"Error: no test specified\" && exit 1",
                "dev": "node index.mjs"
              },
              "dependencies": {
                "@scope/package-b": "1.0.0"
              },
              "keywords": [],
              "author": "",
              "license": "ISC"
        }"#
        .as_bytes(),
    )?;

    monorepo_package_b_json.write_all(
        r#"
        {
            "name": "@scope/package-b",
            "version": "1.0.0",
            "description": "My new package B",
            "main": "index.mjs",
            "repository": {
              "url": "git+ssh://git@github.com/websublime/workspace-node-binding-tools.git",
              "type": "git"
            },
            "scripts": {
              "test": "echo \"Error: no test specified\" && exit 1",
              "dev": "node index.mjs"
            },
            "keywords": [],
            "author": "",
            "license": "ISC"
        }"#
        .as_bytes(),
    )?;

    match package_manager {
        PackageManager::Yarn => {
            let yarn_lock = monorepo_temp_dir.join("yarn.lock");
            File::create(&yarn_lock)?;
        }
        PackageManager::Pnpm => {
            let pnpm_lock = monorepo_temp_dir.join("pnpm-lock.yaml");
            let pnpm_workspace = monorepo_temp_dir.join("pnpm-workspace.yaml");

            let mut lock_file = File::create(&pnpm_lock)?;
            lock_file.write_all(r#"lockfileVersion: '9.0'"#.as_bytes())?;

            let mut workspace_file = File::create(&pnpm_workspace)?;
            workspace_file.write_all(
                r#"
                packages:
                  - "packages/*"
            "#
                .as_bytes(),
            )?;
        }
        PackageManager::Bun => {
            let bun_lock = monorepo_temp_dir.join("bun.lockb");
            File::create(&bun_lock)?;
        }
        PackageManager::Npm => {
            let npm_lock = monorepo_temp_dir.join("package-lock.json");
            File::create(&npm_lock)?;
        }
    }

    let init = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("init")
        .arg("--initial-branch")
        .arg("main")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git init problem");

    init.wait_with_output()?;

    let config_email = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("config")
        .arg("user.email")
        .arg("machine@websublime.dev")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git config user email problem");

    config_email.wait_with_output()?;

    let config_name = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("config")
        .arg("user.name")
        .arg("Sublime Machine")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git config user name problem");

    config_name.wait_with_output()?;

    let add = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("add")
        .arg(".")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git add problem");

    add.wait_with_output()?;

    let commit = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("commit")
        .arg("-m")
        .arg("feat: project creation")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git commit problem");

    commit.wait_with_output()?;

    let tag_a = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("tag")
        .arg("-a")
        .arg("@scope/package-a@1.0.0")
        .arg("-m")
        .arg("chore: release package-a@1.0.0")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git tag problem");

    tag_a.wait_with_output()?;

    let tag_b = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("tag")
        .arg("-a")
        .arg("@scope/package-b@1.0.0")
        .arg("-m")
        .arg("chore: release package-b@1.0.0")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git tag problem");

    tag_b.wait_with_output()?;

    let canonic_path = &std::fs::canonicalize(Path::new(&monorepo_temp_dir)).unwrap();
    let root = canonic_path.as_path().display().to_string();

    Ok(PathBuf::from(root))
}
