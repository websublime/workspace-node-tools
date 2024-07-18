#![warn(dead_code)]
#![warn(unused_imports)]
#![allow(clippy::all)]

use package_json_schema::PackageJson;
use semver::{BuildMetadata, Prerelease, Version as SemVersion};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;

use super::conventional::ConventionalPackage;
use super::conventional::{get_conventional_for_package, ConventionalPackageOptions};
use super::git::git_current_sha;
use super::git::git_fetch_all;
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
    packages: Vec<String>,
    release_as: Bump,
    fetch_all: Option<bool>,
    fetch_tags: Option<bool>,
    pub cwd: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BumpOptions {
    packages: Vec<String>,
    release_as: Bump,
    fetch_all: Option<bool>,
    fetch_tags: Option<bool>,
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

        let ref version = semversion.to_string();
        package.update_version(version.to_string());

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

        bumps.push(bump.clone());

        // TODO: sync need to update dependency version

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
