#![warn(dead_code)]
#![warn(unused_imports)]
#![allow(clippy::all)]

//! # Bumps
//!
//! This module is responsible for managing the bumps in the monorepo.
use package_json_schema::PackageJson;
use semver::{BuildMetadata, Prerelease, Version as SemVersion};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;

use super::conventional::ConventionalPackage;
use super::conventional::{get_conventional_for_package, ConventionalPackageOptions};
use super::git::{git_all_files_changed_since_sha, git_current_sha, git_fetch_all};
use super::packages::get_packages;
use super::packages::PackageInfo;
use super::paths::get_project_root_path;

#[cfg(feature = "napi")]
#[napi(string_enum)]
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum Bump {
    Major,
    Minor,
    Patch,
    Snapshot,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq)]
pub enum Bump {
    Major,
    Minor,
    Patch,
    Snapshot,
}

#[cfg(feature = "napi")]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BumpOptions {
    pub packages: Vec<String>,
    pub release_as: Bump,
    pub fetch_all: Option<bool>,
    pub fetch_tags: Option<bool>,
    pub cwd: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BumpOptions {
    pub packages: Vec<String>,
    pub release_as: Bump,
    pub fetch_all: Option<bool>,
    pub fetch_tags: Option<bool>,
    pub cwd: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BumpPackage {
    pub from: String,
    pub to: String,
    pub release_as: Bump,
    pub conventional: ConventionalPackage,
}

#[cfg(feature = "napi")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BumpPackage {
    pub from: String,
    pub to: String,
    pub release_as: Bump,
    pub conventional: ConventionalPackage,
}

impl Bump {
    fn bump_major(version: String) -> SemVersion {
        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.major += 1;
        sem_version.minor = 0;
        sem_version.patch = 0;
        sem_version.pre = Prerelease::EMPTY;
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }

    fn bump_minor(version: String) -> SemVersion {
        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.minor += 1;
        sem_version.patch = 0;
        sem_version.pre = Prerelease::EMPTY;
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }

    fn bump_patch(version: String) -> SemVersion {
        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.patch += 1;
        sem_version.pre = Prerelease::EMPTY;
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }

    fn bump_snapshot(version: String) -> SemVersion {
        let sha = git_current_sha(None);
        let alpha = format!("alpha.{}", sha);

        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.pre = Prerelease::new(alpha.as_str()).unwrap_or(Prerelease::EMPTY);
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }
}

pub fn sync_bumps(bump_package: &BumpPackage, cwd: Option<String>) -> Vec<String> {
    get_packages(cwd)
        .iter()
        .filter(|package| {
            let pkg_json: PackageJson =
                serde_json::from_value(package.pkg_json.to_owned()).unwrap();

            if pkg_json.dependencies.is_some() {
                let dependencies = pkg_json.dependencies.unwrap();
                return dependencies.contains_key(&bump_package.conventional.package_info.name);
            }

            if pkg_json.dev_dependencies.is_some() {
                let dev_dependencies = pkg_json.dev_dependencies.unwrap();
                return dev_dependencies.contains_key(&bump_package.conventional.package_info.name);
            }

            false
        })
        .map(|package| package.name.to_string())
        .collect::<Vec<String>>()
}

pub fn get_bumps(options: BumpOptions) -> Vec<BumpPackage> {
    let ref root = match options.cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let release_as = options.release_as.to_owned();
    let mut bumps: Vec<BumpPackage> = vec![];

    if options.fetch_tags.is_some() {
        git_fetch_all(Some(root.to_string()), options.fetch_tags)
            .expect("No possible to fetch tags");
    }

    let packages = get_packages(Some(root.to_string()))
        .iter()
        .filter(|package| options.packages.contains(&package.name))
        .map(|package| package.to_owned())
        .collect::<Vec<PackageInfo>>();

    if packages.len() == 0 {
        return bumps;
    }

    for mut package in packages {
        let package_version = &package.version.to_string();
        let changelog_exists =
            Path::new(&format!("{}/CHANGELOG.md", package.package_path)).exists();

        let semversion = match release_as {
            Bump::Major => Bump::bump_major(package_version.to_string()),
            Bump::Minor => Bump::bump_minor(package_version.to_string()),
            Bump::Patch => Bump::bump_patch(package_version.to_string()),
            Bump::Snapshot => Bump::bump_snapshot(package_version.to_string()),
        };

        let title = match changelog_exists {
            true => None,
            false => Some("# What changed?".to_string()),
        };

        let changed_files =
            git_all_files_changed_since_sha(String::from("main"), Some(root.to_string()));
        let ref version = semversion.to_string();

        package.update_version(version.to_string());
        package.extend_changed_files(changed_files);

        let conventional = get_conventional_for_package(
            &package,
            options.fetch_all,
            Some(root.to_string()),
            &Some(ConventionalPackageOptions {
                version: Some(version.to_string()),
                title,
            }),
        );

        let bump = BumpPackage {
            from: package_version.to_string(),
            to: version.to_string(),
            release_as,
            conventional,
        };
        bumps.push(bump.to_owned());

        let sync_packages = sync_bumps(&bump, Some(root.to_string()));

        if sync_packages.len() > 0 {
            let sync_bumps = get_bumps(BumpOptions {
                packages: sync_packages,
                release_as: Bump::Patch,
                fetch_all: options.fetch_all,
                fetch_tags: options.fetch_tags,
                cwd: Some(root.to_string()),
            });

            bumps.extend(sync_bumps);
        }
    }

    bumps
}

pub fn apply_bumps(options: BumpOptions) -> Vec<BumpPackage> {
    let bumps = get_bumps(options);

    if bumps.len() != 0 {
        for _bump in &bumps {
            todo!("Apply bump to the package");
        }
    }

    bumps
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manager::PackageManager;
    use crate::packages::get_changed_packages;
    use crate::paths::get_project_root_path;
    use crate::utils::create_test_monorepo;
    use std::fs::remove_dir_all;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    fn create_package_change(monorepo_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let js_path = monorepo_dir.join("packages/package-b/index.js");

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
    fn test_get_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_package_change(monorepo_dir)?;

        let ref root = project_root.unwrap().to_string();

        let packages = get_changed_packages(Some(String::from("main")), Some(root.to_string()))
            .iter()
            .map(|package| package.name.to_string())
            .collect::<Vec<String>>();

        let bumps = get_bumps(BumpOptions {
            packages,
            release_as: Bump::Minor,
            fetch_all: None,
            fetch_tags: None,
            cwd: Some(root.to_string()),
        });

        assert_eq!(bumps.len(), 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
