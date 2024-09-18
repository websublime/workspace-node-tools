#![warn(dead_code)]
#![warn(unused_imports)]
#![allow(clippy::all)]

//! # Bumps
//!
//! This module is responsible for managing the bumps in the monorepo.
use semver::{BuildMetadata, Prerelease, Version as SemVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::conventional::ConventionalPackage;
use crate::packages::get_changed_packages;

use super::changes::{get_package_change, Change};
use super::conventional::{get_conventional_for_package, ConventionalPackageOptions};
use super::git::{
    git_add, git_all_files_changed_since_sha, git_commit, git_current_branch, git_current_sha,
    git_fetch_all,
};
use super::packages::PackageInfo;
use super::packages::{get_package_info, get_packages, DependencyInfo};
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

pub fn get_bumps(options: &BumpOptions) -> Vec<BumpPackage> {
    let ref root = match options.cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    //let mut bumps: Vec<BumpPackage> = vec![];

    if options.fetch_tags.is_some() {
        git_fetch_all(Some(root.to_string()), options.fetch_tags)
            .expect("No possible to fetch tags");
    }

    let since = options.since.clone().unwrap_or(String::from("main"));

    let ref packages = get_packages(Some(root.to_string()));
    let changed_packages = get_changed_packages(Some(since.to_string()), Some(root.to_string()));

    if changed_packages.len() == 0 {
        return vec![];
    }

    let mut bump_changes = HashMap::new();

    for changed_package in changed_packages.iter() {
        let change = options
            .changes
            .iter()
            .find(|change| change.package == changed_package.name);

        if change.is_some() {
            bump_changes.insert(changed_package.name.to_string(), change.unwrap().to_owned());
        }

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

            return bump;
        })
        .collect::<Vec<BumpPackage>>();

    bumps.iter_mut().for_each(|bump| {
        bumps.iter_mut().for_each(|b| {
            let deps = b.package_info.dependencies.to_owned();
            deps.iter().for_each(|dependency| {
                if dependency.name == bump.package_info.name {
                    b.package_info.update_dependency_version(
                        bump.package_info.name.to_string(),
                        bump.to.to_string(),
                    );

                    b.package_info.update_dev_dependency_version(
                        bump.package_info.name.to_string(),
                        bump.to.to_string(),
                    );
                }
            });
        });

        let version = bump.to.to_string();
        bump.package_info.update_version(version.to_string());
    });

    /*if changed_packages_dependencies.len() > 0 {
        for package in changed_packages_dependencies.iter() {
            let is_changed = options
                .changes
                .iter()
                .any(|change| change.package == package.name);

            if is_changed {
                continue;
            }

            let changes = match changes_dependencies.get(&package.name) {
                Some(change) => change.to_owned(),
                None => Change {
                    package: package.name.to_string(),
                    release_as: Bump::Patch,
                    deploy: vec![],
                },
            };
            let recommended_bump = get_package_recommend_bump(
                &package,
                root,
                Some(BumpOptions {
                    changes: vec![changes],
                    since: Some(String::from("main")),
                    release_as: Some(Bump::Patch),
                    fetch_all: options.fetch_all.to_owned(),
                    fetch_tags: options.fetch_tags.to_owned(),
                    sync_deps: Some(true),
                    push: options.push.to_owned(),
                    cwd: Some(root.to_string()),
                }),
            );

            let mut bump = BumpPackage {
                from: recommended_bump.from.to_string(),
                to: recommended_bump.to.to_string(),
                conventional_commits: recommended_bump
                    .conventional
                    .conventional_commits
                    .to_owned(),
                package_info: recommended_bump.package_info.to_owned(),
            };

            bump.package_info
                .update_version(recommended_bump.to.to_string());

            bumps.push(bump);
        }
    }

    for package in changed_packages.iter() {
        let change = options
            .changes
            .clone()
            .into_iter()
            .find(|change| change.package == package.name);
        let recommended_bump = get_package_recommend_bump(
            &package,
            root,
            Some(BumpOptions {
                changes: vec![change.unwrap_or(Change {
                    package: package.name.to_string(),
                    release_as: Bump::Patch,
                    deploy: vec![],
                })],
                since: Some(since.to_string()),
                release_as: options.release_as.to_owned(),
                fetch_all: options.fetch_all.to_owned(),
                fetch_tags: options.fetch_tags.to_owned(),
                sync_deps: options.sync_deps.to_owned(),
                push: options.push.to_owned(),
                cwd: Some(root.to_string()),
            }),
        );

        let mut bump = BumpPackage {
            from: recommended_bump.from.to_string(),
            to: recommended_bump.to.to_string(),
            conventional_commits: recommended_bump
                .conventional
                .conventional_commits
                .to_owned(),
            package_info: recommended_bump.package_info.to_owned(),
        };

        bump.package_info
            .update_version(recommended_bump.to.to_string());

        bumps.iter_mut().for_each(|bump_dep| {
            let exist = bump_dep
                .package_info
                .dependencies
                .iter()
                .any(|dep| dep.name == bump.package_info.name);

            if exist {
                bump_dep.package_info.update_dependency_version(
                    bump.package_info.name.to_string(),
                    bump.to.to_string(),
                );
                bump_dep.package_info.update_dev_dependency_version(
                    bump.package_info.name.to_string(),
                    bump.to.to_string(),
                );
            }
        });

        bumps.push(bump);
    }*/

    bumps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::{add_change, get_change, init_changes};
    use crate::manager::PackageManager;
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

        dbg!(&bumps);

        assert_eq!(bumps.len(), 2);
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

        dbg!(&bumps);

        assert_eq!(bumps.len(), 3);
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

        dbg!(&bumps);

        assert_eq!(3, 3);
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

        dbg!(&bumps);

        assert_eq!(3, 3);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
/*pub fn get_packages_graph(root: &String) {
    let monorepo_packages = get_packages(Some(root.to_string()));
    let packages_names = monorepo_packages.iter().map(|package| package.name.to_string()).collect::<Vec<String>>();

    monorepo_packages.iter().map(|package| {
        let package_json_map = serde_json::Map::new();
        let mut package_json_map = package_json_map.clone();
        package_json_map.clone_from(package.pkg_json.as_object().unwrap());


    });
}

pub fn get_deps_graph_for_package(package: &PackageInfo, root: &String) -> Vec<PackageInfo> {
    let monorepo_packages = get_packages(Some(root.to_string()));

    let packages = monorepo_packages
        .iter()
        .filter(|p| {
            let package_json_map = serde_json::Map::new();
            let mut package_json_map = package_json_map.clone();
            package_json_map.clone_from(p.pkg_json.as_object().unwrap());

            if package_json_map.contains_key("dependencies") {
                let dependencies_value = package_json_map.get_mut("dependencies").unwrap();
                let dependencies_value = dependencies_value.as_object_mut().unwrap();
                let has_dependency = dependencies_value.contains_key(&package.name);

                return match has_dependency {
                    true => {
                        let dep_version = Value::String(
                            dependencies_value[&package.name.to_string()].to_string(),
                        )
                        .to_string();
                        let is_internal = dep_version.contains("*");

                        if is_internal {
                            return false;
                        }

                        return true;
                    }
                    false => false,
                };
            }

            if package_json_map.contains_key("devDependencies") {
                let dev_dependencies_value = package_json_map.get_mut("devDependencies").unwrap();
                let dev_dependencies_value = dev_dependencies_value.as_object_mut().unwrap();
                let has_dependency = dev_dependencies_value.contains_key(&package.name);

                return match has_dependency {
                    true => {
                        let dep_version = Value::String(
                            dev_dependencies_value[&package.name.to_string()].to_string(),
                        )
                        .to_string();
                        let is_internal = dep_version.contains("*");

                        if is_internal {
                            return false;
                        }

                        return true;
                    }
                    false => false,
                };
            }

            false
        })
        .map(|package| package.to_owned())
        .collect::<Vec<PackageInfo>>();

    packages
}

// Bumps packages with to new versions.
pub fn get_bumps(options: BumpOptions) -> Vec<BumpPackage> {
    let ref root = match options.cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let ref since = match options.since {
        Some(ref since) => since.to_string(),
        None => String::from("main"),
    };

    let ref current_branch =
        git_current_branch(Some(root.to_string())).unwrap_or(String::from("main"));

    let mut bumps: Vec<BumpPackage> = vec![];

    if options.fetch_tags.is_some() {
        git_fetch_all(Some(root.to_string()), options.fetch_tags)
            .expect("No possible to fetch tags");
    }

    let packages = get_packages(Some(root.to_string()))
        .iter()
        .filter(|package| {
            options
                .changes
                .iter()
                .any(|change| change.package.eq(&package.name))
        })
        .map(|package| package.to_owned())
        .collect::<Vec<PackageInfo>>();

    if packages.len() == 0 {
        return bumps;
    }

    for mut package in packages {
        let package_version = &package.version.to_string();
        let package_name = &package.name.to_string();
        let package_change = get_package_change(
            package_name.to_string(),
            current_branch.to_string(),
            Some(root.to_string()),
        );

        let bumped = bumps.iter().find(|b| b.package_info.name.eq(package_name));

        if bumped.is_some() {
            continue;
        }

        let release_as = options
            .release_as
            .unwrap_or_else(|| match package_change.to_owned() {
                Some(change) => change.release_as,
                None => Bump::Patch,
            });

        let deploy_to = match package_change.to_owned() {
            Some(change) => change.deploy,
            None => vec![String::from("production")],
        };

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
            conventional_commits: conventional.conventional_commits.to_owned(),
            package_info: package.to_owned(),
        };

        bumps.push(bump.to_owned());

        if options.sync_deps.unwrap_or(false) {
            let mut deps_graph = get_deps_graph_for_package(&package, root);

            if deps_graph.len() == 0 {
                continue;
            }

            let deps_changes = deps_graph
                .iter_mut()
                .map(|dep| {
                    dep.update_dependency_version(package.name.to_string(), version.to_string());
                    dep.update_dev_dependency_version(
                        package.name.to_string(),
                        version.to_string(),
                    );
                    dep.extend_changed_files(vec![String::from("package.json")]);
                    dep.write_package_json();

                    git_add(&root.to_string(), &dep.package_json_path.to_owned())
                        .expect("Failed to add package.json");
                    git_commit(
                        format!(
                            "chore: update dependency {} in {}",
                            package.name.to_string(),
                            dep.name.to_string()
                        ),
                        None,
                        None,
                        Some(root.to_string()),
                    )
                    .expect("Failed to commit package.json");

                    Change {
                        package: dep.name.to_string(),
                        release_as: dependency_semversion,
                        deploy: deploy_to.to_owned(),
                    }
                })
                .collect::<Vec<Change>>();

            let sync_bumps = get_bumps(BumpOptions {
                changes: deps_changes,
                since: Some(since.to_string()),
                release_as: Some(dependency_semversion),
                fetch_all: options.fetch_all,
                fetch_tags: options.fetch_tags,
                sync_deps: Some(true),
                push: Some(false),
                cwd: Some(root.to_string()),
            });

            bumps.extend(sync_bumps);
        }
    }

    bumps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::{add_change, get_change, init_changes};
    use crate::manager::PackageManager;
    use crate::paths::get_project_root_path;
    use crate::utils::create_test_monorepo;
    use std::fs::remove_dir_all;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

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

    // Current debug tests

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

    #[test]
    fn test_single_get_bumps() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm).unwrap();
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf())).unwrap();

        let ref root = project_root.to_string();

        create_single_package(monorepo_dir)?;
        create_single_changes(&root)?;

        let changes = get_change(String::from("feat/message"), Some(root.to_string()));

        let bumps = get_bumps(BumpOptions {
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

        let bumps = get_bumps(BumpOptions {
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

        let bumps = get_bumps(BumpOptions {
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

        let bumps = get_bumps(BumpOptions {
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
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}*/
