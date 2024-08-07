#![warn(dead_code)]
#![warn(unused_imports)]
#![allow(clippy::all)]

//! # Bumps
//!
//! This module is responsible for managing the bumps in the monorepo.
use semver::{BuildMetadata, Prerelease, Version as SemVersion};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use super::changes::init_changes;
use super::conventional::ConventionalPackage;
use super::conventional::{get_conventional_for_package, ConventionalPackageOptions};
use super::git::{
    git_add, git_add_all, git_all_files_changed_since_sha, git_commit, git_config, git_current_sha,
    git_fetch_all, git_push, git_tag,
};
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
/// Enum representing the type of bump to be performed.
pub enum Bump {
    Major,
    Minor,
    Patch,
    Snapshot,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BumpOptions {
    pub packages: Vec<String>,
    pub since: Option<String>,
    pub release_as: Bump,
    pub fetch_all: Option<bool>,
    pub fetch_tags: Option<bool>,
    pub sync_deps: Option<bool>,
    pub push: Option<bool>,
    pub cwd: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
/// Struct representing the options for the bump operation.
pub struct BumpOptions {
    pub packages: Vec<String>,
    pub since: Option<String>,
    pub release_as: Bump,
    pub fetch_all: Option<bool>,
    pub fetch_tags: Option<bool>,
    pub sync_deps: Option<bool>,
    pub push: Option<bool>,
    pub cwd: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
/// Struct representing the bump package.
pub struct BumpPackage {
    pub from: String,
    pub to: String,
    pub release_as: Bump,
    pub conventional: ConventionalPackage,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BumpPackage {
    pub from: String,
    pub to: String,
    pub release_as: Bump,
    pub conventional: ConventionalPackage,
}

impl Bump {
    /// Bumps the version of the package to major.
    fn bump_major(version: String) -> SemVersion {
        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.major += 1;
        sem_version.minor = 0;
        sem_version.patch = 0;
        sem_version.pre = Prerelease::EMPTY;
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }

    /// Bumps the version of the package to minor.
    fn bump_minor(version: String) -> SemVersion {
        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.minor += 1;
        sem_version.patch = 0;
        sem_version.pre = Prerelease::EMPTY;
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }

    /// Bumps the version of the package to patch.
    fn bump_patch(version: String) -> SemVersion {
        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.patch += 1;
        sem_version.pre = Prerelease::EMPTY;
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }

    /// Bumps the version of the package to snapshot appending the sha to the version.
    fn bump_snapshot(version: String) -> SemVersion {
        let sha = git_current_sha(None);
        let duration_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();
        let alpha = format!("alpha.{}.{}", timestamp_nanos, sha);

        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.pre = Prerelease::new(alpha.as_str()).unwrap_or(Prerelease::EMPTY);
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }
}

/// Bumps the version of dev-dependencies and dependencies.
pub fn sync_bumps(bump_package: &BumpPackage, cwd: Option<String>) -> Vec<String> {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    get_packages(Some(root.to_string()))
        .iter()
        .filter(|package| {
            let mut package_json_map = serde_json::Map::new();
            package_json_map.clone_from(package.pkg_json.as_object().unwrap());

            if package_json_map.contains_key("dependencies") {
                let dependencies_value = package_json_map.get_mut("dependencies").unwrap();
                let dependencies_value = dependencies_value.as_object_mut().unwrap();
                let has_dependency =
                    dependencies_value.contains_key(&bump_package.conventional.package_info.name);

                if has_dependency {
                    dependencies_value
                        .entry(bump_package.conventional.package_info.name.to_string())
                        .and_modify(|version| *version = json!(bump_package.to.to_string()));

                    package_json_map["dependencies"] = json!(dependencies_value);

                    let file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&package.package_json_path)
                        .unwrap();
                    let writer = BufWriter::new(&file);
                    serde_json::to_writer_pretty(writer, &package_json_map).unwrap();

                    git_add(&root.to_string(), &package.package_json_path.to_owned())
                        .expect("Failed to add package.json");
                    git_commit(
                        format!(
                            "chore: update dependency {} in {}",
                            bump_package.conventional.package_info.name.to_string(),
                            package.name.to_string()
                        ),
                        None,
                        None,
                        Some(root.to_string()),
                    )
                    .expect("Failed to commit package.json");
                }

                return has_dependency;
            }

            if package_json_map.contains_key("devDependencies") {
                let dev_dependencies_value = package_json_map.get_mut("devDependencies").unwrap();
                let dev_dependencies_value = dev_dependencies_value.as_object_mut().unwrap();
                let has_dependency = dev_dependencies_value
                    .contains_key(&bump_package.conventional.package_info.name);

                if has_dependency {
                    dev_dependencies_value
                        .entry(bump_package.conventional.package_info.name.to_string())
                        .and_modify(|version| *version = json!(bump_package.to.to_string()));

                    package_json_map["devDependencies"] = json!(dev_dependencies_value);

                    let file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&package.package_json_path)
                        .unwrap();
                    let writer = BufWriter::new(&file);
                    serde_json::to_writer_pretty(writer, &package_json_map).unwrap();

                    git_add(&root.to_string(), &package.package_json_path.to_owned())
                        .expect("Failed to add package.json");
                    git_commit(
                        format!(
                            "chore: update devDependency {} in {}",
                            bump_package.conventional.package_info.name.to_string(),
                            package.name.to_string()
                        ),
                        None,
                        None,
                        Some(root.to_string()),
                    )
                    .expect("Failed to commit package.json");
                }

                return has_dependency;
            }

            false
        })
        .map(|package| package.name.to_string())
        .collect::<Vec<String>>()
}

/// Get bumps version of the package. If sync_deps is true, it will also sync the dependencies and dev-dependencies.
/// It will also commit the changes to git.
pub fn get_bumps(options: BumpOptions) -> Vec<BumpPackage> {
    let ref root = match options.cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let ref since = match options.since {
        Some(ref since) => since.to_string(),
        None => String::from("main"),
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

        let semversion = match release_as {
            Bump::Major => Bump::bump_major(package_version.to_string()),
            Bump::Minor => Bump::bump_minor(package_version.to_string()),
            Bump::Patch => Bump::bump_patch(package_version.to_string()),
            Bump::Snapshot => Bump::bump_snapshot(package_version.to_string()),
        };

        let dependency_semversion = match release_as {
            Bump::Snapshot => Bump::Snapshot,
            _ => Bump::Patch,
        };

        let changed_files =
            git_all_files_changed_since_sha(since.to_string(), Some(root.to_string()));
        let ref version = semversion.to_string();

        package.update_version(version.to_string());
        package.extend_changed_files(changed_files);

        let conventional = get_conventional_for_package(
            &package,
            options.fetch_all,
            Some(root.to_string()),
            &Some(ConventionalPackageOptions {
                version: Some(version.to_string()),
                title: Some("# What changed?".to_string()),
            }),
        );

        let bump = BumpPackage {
            from: package_version.to_string(),
            to: version.to_string(),
            release_as,
            conventional,
        };
        bumps.push(bump.to_owned());

        if options.sync_deps.unwrap_or(false) {
            let sync_packages = sync_bumps(&bump, Some(root.to_string()));

            if sync_packages.len() > 0 {
                let sync_bumps = get_bumps(BumpOptions {
                    packages: sync_packages,
                    since: Some(since.to_string()),
                    release_as: dependency_semversion,
                    fetch_all: options.fetch_all,
                    fetch_tags: options.fetch_tags,
                    sync_deps: Some(true),
                    push: Some(false),
                    cwd: Some(root.to_string()),
                });

                bumps.extend(sync_bumps);
            }
        }
    }

    bumps
}

/// Apply version bumps, commit and push changes. Returns a list of packages that have been updated.
/// Also generate changelog file and update dependencies and devDependencies in package.json.
pub fn apply_bumps(options: BumpOptions) -> Vec<BumpPackage> {
    let ref root = match options.cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let ref changes_data = init_changes(Some(root.to_string()), &None);
    let git_user_name = changes_data.git_user_name.to_owned();
    let git_user_email = changes_data.git_user_email.to_owned();

    git_config(
        &git_user_name.unwrap_or(String::from("")),
        &git_user_email.unwrap_or(String::from("")),
        &root.to_string(),
    )
    .expect("Failed to set git user name and email");

    let bumps = get_bumps(options.to_owned());

    if bumps.len() != 0 {
        for bump in &bumps {
            let git_message = changes_data.message.to_owned();

            let ref bump_pkg_json_file_path =
                PathBuf::from(bump.conventional.package_info.package_json_path.to_string());
            let ref bump_changelog_file_path =
                PathBuf::from(bump.conventional.package_info.package_path.to_string())
                    .join(String::from("CHANGELOG.md"));

            // Write bump_pkg_json_file_path
            let bump_pkg_json_file = OpenOptions::new()
                .write(true)
                .append(false)
                .open(bump_pkg_json_file_path)
                .unwrap();
            let pkg_json_writer = BufWriter::new(bump_pkg_json_file);
            serde_json::to_writer_pretty(pkg_json_writer, &bump.conventional.package_info.pkg_json)
                .unwrap();

            // Write bump_changelog_file_path
            let mut bump_changelog_file = OpenOptions::new()
                .write(true)
                .create(true)
                .append(false)
                .open(bump_changelog_file_path)
                .unwrap();

            bump_changelog_file
                .write_all(bump.conventional.changelog_output.as_bytes())
                .unwrap();

            let ref package_tag = format!("{}@{}", bump.conventional.package_info.name, bump.to);

            git_add_all(&root.to_string()).expect("Failed to add all files to git");
            git_commit(
                git_message.unwrap_or(String::from("chore: release version")),
                None,
                None,
                Some(root.to_string()),
            )
            .unwrap();
            git_tag(
                package_tag.to_string(),
                Some(format!(
                    "chore: release {} to version {}",
                    bump.conventional.package_info.name, bump.to
                )),
                Some(root.to_string()),
            )
            .unwrap();

            if options.push.unwrap_or(false) {
                git_push(Some(root.to_string()), Some(true)).unwrap();
            }
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
            since: Some(String::from("main")),
            release_as: Bump::Minor,
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(true),
            push: Some(false),
            cwd: Some(root.to_string()),
        });

        assert_eq!(bumps.len(), 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_apply_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_package_change(monorepo_dir)?;

        let ref root = project_root.unwrap().to_string();

        let packages = get_changed_packages(Some(String::from("main")), Some(root.to_string()))
            .iter()
            .map(|package| package.name.to_string())
            .collect::<Vec<String>>();

        let main_branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("main")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git checkout main problem");

        main_branch.wait_with_output()?;

        let merge_branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("merge")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git merge problem");

        merge_branch.wait_with_output()?;

        let bump_options = BumpOptions {
            packages,
            since: Some(String::from("main")),
            release_as: Bump::Minor,
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(true),
            push: Some(false),
            cwd: Some(root.to_string()),
        };

        let bumps = apply_bumps(bump_options);

        assert_eq!(bumps.len(), 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_snapshot_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_package_change(monorepo_dir)?;

        let ref root = project_root.unwrap().to_string();

        let packages = get_changed_packages(Some(String::from("main")), Some(root.to_string()))
            .iter()
            .map(|package| package.name.to_string())
            .collect::<Vec<String>>();

        let main_branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("main")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git checkout main problem");

        main_branch.wait_with_output()?;

        let merge_branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("merge")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git merge problem");

        merge_branch.wait_with_output()?;

        let bump_options = BumpOptions {
            packages,
            since: Some(String::from("main")),
            release_as: Bump::Snapshot,
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(true),
            push: Some(false),
            cwd: Some(root.to_string()),
        };

        let bumps = apply_bumps(bump_options);

        assert_eq!(bumps.len(), 2);
        assert_ne!(&bumps.get(0).unwrap().to, &bumps.get(1).unwrap().to);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
