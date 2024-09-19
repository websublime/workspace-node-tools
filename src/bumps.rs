#![warn(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::all)]

//! # Bumps
//!
//! This module is responsible for managing the bumps in the monorepo.
use semver::{BuildMetadata, Prerelease, Version as SemVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::conventional::ConventionalPackage;

use super::changes::{get_package_change, init_changes, Change};
use super::conventional::{get_conventional_for_package, ConventionalPackageOptions};
use super::git::{
    git_add_all, git_all_files_changed_since_sha, git_commit, git_config, git_current_branch,
    git_current_sha, git_fetch_all, git_push, git_tag,
};
use super::packages::PackageInfo;
use super::packages::{get_package_info, get_packages};
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
    pub changes: Vec<Change>,
    pub since: Option<String>,
    pub release_as: Option<Bump>,
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
    pub changes: Vec<Change>,
    pub since: Option<String>,
    pub release_as: Option<Bump>,
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
    pub package_info: PackageInfo,
    pub conventional_commits: Value,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BumpPackage {
    pub from: String,
    pub to: String,
    pub package_info: PackageInfo,
    pub conventional_commits: Value,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
/// Struct representing the bump package.
pub struct RecommendBumpPackage {
    pub from: String,
    pub to: String,
    pub package_info: PackageInfo,
    pub conventional: ConventionalPackage,
    pub changed_files: Vec<String>,
    pub deploy_to: Vec<String>,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
/// Struct representing the bump package.
pub struct RecommendBumpPackage {
    pub from: String,
    pub to: String,
    pub package_info: PackageInfo,
    pub conventional: ConventionalPackage,
    pub changed_files: Vec<String>,
    pub deploy_to: Vec<String>,
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
        let alpha = format!("alpha.{}.{}", 0, sha);

        let mut sem_version = SemVersion::parse(&version).unwrap();
        sem_version.pre = Prerelease::new(alpha.as_str()).unwrap_or(Prerelease::EMPTY);
        sem_version.build = BuildMetadata::EMPTY;
        sem_version
    }
}

pub fn get_package_recommend_bump(
    package_info: &PackageInfo,
    root: &String,
    options: Option<BumpOptions>,
) -> RecommendBumpPackage {
    let ref current_branch =
        git_current_branch(Some(root.to_string())).unwrap_or(String::from("origin/main"));

    let package_version = &package_info.version.to_string();
    let package_name = &package_info.name.to_string();
    let package_change = get_package_change(
        package_name.to_string(),
        current_branch.to_string(),
        Some(root.to_string()),
    );

    let settings = options.unwrap_or_else(|| BumpOptions {
        changes: vec![],
        since: None,
        release_as: None,
        fetch_all: None,
        fetch_tags: None,
        sync_deps: None,
        push: None,
        cwd: None,
    });

    let ref since = settings.since.unwrap_or(String::from("origin/main"));

    let release_as = settings
        .release_as
        .unwrap_or_else(|| match package_change.to_owned() {
            Some(change) => change.release_as,
            None => Bump::Patch,
        });

    let deploy_to = match package_change.to_owned() {
        Some(change) => change.deploy,
        None => vec![String::from("production")],
    };

    let fetch_all = settings.fetch_all.unwrap_or(false);

    let semversion = match release_as {
        Bump::Major => Bump::bump_major(package_version.to_string()),
        Bump::Minor => Bump::bump_minor(package_version.to_string()),
        Bump::Patch => Bump::bump_patch(package_version.to_string()),
        Bump::Snapshot => Bump::bump_snapshot(package_version.to_string()),
    };

    let changed_files = git_all_files_changed_since_sha(since.to_string(), Some(root.to_string()));
    let ref version = semversion.to_string();

    let conventional = get_conventional_for_package(
        &package_info,
        Some(fetch_all),
        Some(root.to_string()),
        &Some(ConventionalPackageOptions {
            version: Some(version.to_string()),
            title: Some("# What changed?".to_string()),
        }),
    );

    RecommendBumpPackage {
        from: package_version.to_string(),
        to: version.to_string(),
        package_info: package_info.to_owned(),
        conventional: conventional.to_owned(),
        changed_files: changed_files.to_owned(),
        deploy_to: deploy_to.to_owned(),
    }
}

/// Get bumps version of the package. If sync_deps is true, it will also sync the dependencies and dev-dependencies.
pub fn get_bumps(options: &BumpOptions) -> Vec<BumpPackage> {
    let ref root = match options.cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    if options.fetch_tags.is_some() {
        git_fetch_all(Some(root.to_string()), options.fetch_tags)
            .expect("No possible to fetch tags");
    }

    let since = options.since.clone().unwrap_or(String::from("main"));

    let ref packages = get_packages(Some(root.to_string()));
    let changed_packages = packages
        .iter()
        .filter(|package| {
            options
                .changes
                .iter()
                .any(|change| change.package == package.name)
        })
        .map(|package| package.to_owned())
        .collect::<Vec<PackageInfo>>();
    //let changed_packages = get_changed_packages(Some(since.to_string()), Some(root.to_string()));

    if changed_packages.len() == 0 {
        return vec![];
    }

    let mut bump_changes = HashMap::new();
    let mut bump_dependencies = HashMap::new();

    for changed_package in changed_packages.iter() {
        let change = options
            .changes
            .iter()
            .find(|change| change.package == changed_package.name);

        if change.is_some() {
            bump_changes.insert(changed_package.name.to_string(), change.unwrap().to_owned());
        }

        if options.sync_deps.unwrap_or(false) {
            packages.iter().for_each(|package| {
                package.dependencies.iter().for_each(|dependency| {
                    if dependency.name == changed_package.name {
                        if change.is_some() && !bump_changes.contains_key(&package.name) {
                            bump_changes.insert(
                                package.name.to_string(),
                                Change {
                                    package: package.name.to_string(),
                                    release_as: Bump::Patch,
                                    deploy: change.unwrap().deploy.to_owned(),
                                },
                            );
                        }
                    }
                });
            });
        }
    }

    let mut bumps = bump_changes
        .iter()
        .map(|(package_name, change)| {
            let package = get_package_info(package_name.to_string(), Some(root.to_string()));

            let recommended_bump = get_package_recommend_bump(
                &package.unwrap(),
                root,
                Some(BumpOptions {
                    changes: vec![change.to_owned()],
                    since: Some(since.to_string()),
                    release_as: Some(change.release_as.to_owned()),
                    fetch_all: options.fetch_all.to_owned(),
                    fetch_tags: options.fetch_tags.to_owned(),
                    sync_deps: options.sync_deps.to_owned(),
                    push: options.push.to_owned(),
                    cwd: Some(root.to_string()),
                }),
            );

            let bump = BumpPackage {
                from: recommended_bump.from.to_string(),
                to: recommended_bump.to.to_string(),
                conventional_commits: recommended_bump
                    .conventional
                    .conventional_commits
                    .to_owned(),
                package_info: recommended_bump.package_info.to_owned(),
            };

            if bump.package_info.dependencies.len() > 0 {
                bump_dependencies.insert(
                    package_name.to_string(),
                    bump.package_info.dependencies.to_owned(),
                );
            }

            return bump;
        })
        .collect::<Vec<BumpPackage>>();

    bumps.iter_mut().for_each(|bump| {
        let version = bump.to.to_string();
        bump.package_info.update_version(version.to_string());
        bump.package_info
            .extend_changed_files(vec![String::from("package.json")]);
        bump.package_info.write_package_json();
    });

    if options.sync_deps.unwrap_or(false) {
        bump_dependencies.iter().for_each(|(package_name, deps)| {
            let temp_bumps = bumps.clone();
            let bump = bumps
                .iter_mut()
                .find(|b| b.package_info.name == package_name.to_string())
                .unwrap();

            for dep in deps {
                let bump_dep = temp_bumps.iter().find(|b| b.package_info.name == dep.name);

                if bump_dep.is_some() {
                    bump.package_info.update_dependency_version(
                        dep.name.to_string(),
                        bump_dep.unwrap().to.to_string(),
                    );
                    bump.package_info.update_dev_dependency_version(
                        dep.name.to_string(),
                        bump_dep.unwrap().to.to_string(),
                    );
                    bump.package_info.write_package_json();
                }
            }
        });
    }

    bumps
}

/// Apply version bumps, commit and push changes. Returns a list of packages that have been updated.
/// Also generate changelog file and update dependencies and devDependencies in package.json.
pub fn apply_bumps(options: &BumpOptions) -> Vec<BumpPackage> {
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

    let bumps = get_bumps(options);

    if bumps.len() != 0 {
        for bump in &bumps {
            let git_message = changes_data.message.to_owned();

            let ref bump_pkg_json_file_path =
                PathBuf::from(bump.package_info.package_json_path.to_string());
            let ref bump_changelog_file_path =
                PathBuf::from(bump.package_info.package_path.to_string())
                    .join(String::from("CHANGELOG.md"));

            // Write bump_pkg_json_file_path
            let bump_pkg_json_file = OpenOptions::new()
                .write(true)
                .append(false)
                .open(bump_pkg_json_file_path)
                .unwrap();
            let pkg_json_writer = BufWriter::new(bump_pkg_json_file);
            serde_json::to_writer_pretty(pkg_json_writer, &bump.package_info.pkg_json).unwrap();

            let conventional = get_conventional_for_package(
                &bump.package_info,
                options.fetch_all.to_owned(),
                Some(root.to_string()),
                &Some(ConventionalPackageOptions {
                    version: Some(bump.to.to_string()),
                    title: Some("# What changed?".to_string()),
                }),
            );

            // Write bump_changelog_file_path
            let mut bump_changelog_file = OpenOptions::new()
                .write(true)
                .create(true)
                .append(false)
                .open(bump_changelog_file_path)
                .unwrap();

            bump_changelog_file
                .write_all(conventional.changelog_output.as_bytes())
                .unwrap();

            let ref package_tag = format!("{}@{}", bump.package_info.name, bump.to);

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
                    bump.package_info.name, bump.to
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
    use crate::changes::{add_change, get_change, init_changes};
    use crate::manager::PackageManager;
    use crate::packages::get_changed_packages;
    use crate::paths::get_project_root_path;
    use crate::utils::create_test_monorepo;
    use std::fs::remove_dir_all;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    fn create_single_changes(root: &String) -> Result<(), Box<dyn std::error::Error>> {
        let change_package_a = Change {
            package: String::from("@scope/package-a"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        add_change(&change_package_a, Some(root.to_string()));

        Ok(())
    }

    fn create_single_package(monorepo_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
            .write_all(r#"export const message = "hello package-a";"#.as_bytes())
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

    fn create_multiple_changes(root: &String) -> Result<(), Box<dyn std::error::Error>> {
        let change_package_a = Change {
            package: String::from("@scope/package-a"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        let change_package_c = Change {
            package: String::from("@scope/package-c"),
            release_as: Bump::Minor,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        add_change(&change_package_a, Some(root.to_string()));
        add_change(&change_package_c, Some(root.to_string()));

        Ok(())
    }

    fn create_multiple_packages(monorepo_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let js_path_package_a = monorepo_dir.join("packages/package-a/index.js");
        let js_path_package_c = monorepo_dir.join("packages/package-c/index.js");

        let branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("-b")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git branch problem");

        branch.wait_with_output()?;

        let mut js_file_package_a = File::create(&js_path_package_a)?;
        js_file_package_a
            .write_all(r#"export const message = "hello package-a";"#.as_bytes())
            .unwrap();

        let mut js_file_package_c = File::create(&js_path_package_c)?;
        js_file_package_c
            .write_all(r#"export const message = "hello package-c";"#.as_bytes())
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

    fn create_single_dependency_changes(root: &String) -> Result<(), Box<dyn std::error::Error>> {
        let change_package_a = Change {
            package: String::from("@scope/package-b"),
            release_as: Bump::Snapshot,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        add_change(&change_package_a, Some(root.to_string()));

        Ok(())
    }

    fn create_single_dependency_package(
        monorepo_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
            .write_all(r#"export const message = "hello package-b";"#.as_bytes())
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

    fn create_multiple_dependency_changes(root: &String) -> Result<(), Box<dyn std::error::Error>> {
        let change_package_a = Change {
            package: String::from("@scope/package-a"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        let change_package_b = Change {
            package: String::from("@scope/package-b"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        add_change(&change_package_a, Some(root.to_string()));
        add_change(&change_package_b, Some(root.to_string()));

        Ok(())
    }

    fn create_multiple_dependency_packages(
        monorepo_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let js_path = monorepo_dir.join("packages/package-b/index.js");
        let js_path_no_depend = monorepo_dir.join("packages/package-a/index.js");

        let branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("-b")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git branch problem");

        branch.wait_with_output()?;

        let mut js_file_no_depend = File::create(&js_path_no_depend)?;
        js_file_no_depend
            .write_all(r#"export const message = "hello package-a";"#.as_bytes())
            .unwrap();

        let mut js_file = File::create(&js_path)?;
        js_file
            .write_all(r#"export const message = "hello package-b";"#.as_bytes())
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
    fn test_single_get_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm).unwrap();
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf())).unwrap();

        let ref root = project_root.to_string();

        create_single_package(monorepo_dir)?;
        create_single_changes(&root)?;

        let changes = get_change(String::from("feat/message"), Some(root.to_string()));

        let bumps = get_bumps(&BumpOptions {
            changes,
            since: Some(String::from("main")),
            release_as: Some(Bump::Major),
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(false),
            push: Some(false),
            cwd: Some(root.to_string()),
        });

        assert_eq!(bumps.len(), 1);

        let first_bump = bumps.get(0);

        assert_eq!(first_bump.is_some(), true);

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_multiple_get_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm).unwrap();
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf())).unwrap();

        let ref root = project_root.to_string();

        create_multiple_packages(monorepo_dir)?;
        create_multiple_changes(&root)?;

        let changes = get_change(String::from("feat/message"), Some(root.to_string()));

        let bumps = get_bumps(&BumpOptions {
            changes,
            since: Some(String::from("main")),
            release_as: None,
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(false),
            push: Some(false),
            cwd: Some(root.to_string()),
        });

        assert_eq!(bumps.len(), 2);

        let first_bump = bumps.get(0);
        let second_bump = bumps.get(1);

        assert_eq!(first_bump.is_some(), true);
        assert_eq!(second_bump.is_some(), true);

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_single_dependency_get_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm).unwrap();
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf())).unwrap();

        let ref root = project_root.to_string();

        create_single_dependency_package(monorepo_dir)?;
        create_single_dependency_changes(&root)?;

        let changes = get_change(String::from("feat/message"), Some(root.to_string()));

        let bumps = get_bumps(&BumpOptions {
            changes,
            since: Some(String::from("main")),
            release_as: None,
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(true),
            push: Some(false),
            cwd: Some(root.to_string()),
        });

        assert_eq!(bumps.len(), 2);

        let first_bump = bumps.get(0);
        let second_bump = bumps.get(1);

        assert_eq!(first_bump.is_some(), true);
        assert_eq!(second_bump.is_some(), true);

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_multiple_dependency_get_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm).unwrap();
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf())).unwrap();

        let ref root = project_root.to_string();

        create_multiple_dependency_packages(monorepo_dir)?;
        create_multiple_dependency_changes(&root)?;

        let changes = get_change(String::from("feat/message"), Some(root.to_string()));

        let bumps = get_bumps(&BumpOptions {
            changes,
            since: Some(String::from("main")),
            release_as: None,
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(true),
            push: Some(false),
            cwd: Some(root.to_string()),
        });

        assert_eq!(bumps.len(), 3);

        let first_bump = bumps.get(0);
        let second_bump = bumps.get(1);
        let third_bump = bumps.get(2);

        assert_eq!(first_bump.is_some(), true);
        assert_eq!(second_bump.is_some(), true);
        assert_eq!(third_bump.is_some(), true);

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_apply_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_multiple_dependency_packages(monorepo_dir)?;

        let ref root = project_root.unwrap().to_string();

        let packages = get_changed_packages(Some(String::from("main")), Some(root.to_string()))
            .iter()
            .map(|package| package.name.to_string())
            .collect::<Vec<String>>();

        init_changes(Some(root.to_string()), &None);

        for package in packages {
            let change_package = Change {
                package: package.to_string(),
                release_as: Bump::Major,
                deploy: vec![String::from("production")],
            };

            add_change(&change_package, Some(root.to_string()));
        }

        let changes = get_change(String::from("feat/message"), Some(root.to_string()));

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
            changes,
            since: Some(String::from("main")),
            release_as: Some(Bump::Minor),
            fetch_all: None,
            fetch_tags: None,
            sync_deps: Some(true),
            push: Some(false),
            cwd: Some(root.to_string()),
        };

        let bumps = apply_bumps(&bump_options);

        assert_eq!(bumps.len(), 3);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
