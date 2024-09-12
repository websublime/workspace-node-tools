#![allow(clippy::all)]

//! # Changes
//!
//! This module is responsible for managing the changes in the monorepo.
//! The changes are stored in a `.changes.json` file in the root of the project.
//!
//! # Example
//! ```json
//! {
//!   "message": "chore(release): release new version",
//!   "gitUserName": "Git Bot",
//!   "gitUserEmail": "git.bot@domain.com",
//!   "changes": {
//!       "BRANCH-NAME": [{
//!           "package": "xxx",
//!           "releaseAs": "patch",
//!           "deploy": ["int"]
//!       }],
//!   }
//!}
//!```
use serde::{Deserialize, Serialize};
use std::io::BufWriter;
use std::{
    collections::BTreeMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::bumps::Bump;

use super::git::git_current_branch;
use super::paths::get_project_root_path;

/// Dynamic data structure to store changes
type ChangesData = BTreeMap<String, Vec<Change>>;

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
/// Options to initialize the changes file
pub struct ChangesOptions {
    pub message: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ChangesOptions {
    pub message: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
/// Data structure to store changes file
pub struct ChangesFileData {
    pub message: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
    pub changes: ChangesData,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ChangesFileData {
    pub message: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
    pub changes: ChangesData,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
/// Data structure to store changes
pub struct Changes {
    pub changes: ChangesData,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Changes {
    pub changes: ChangesData,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
/// Data structure to store a change
pub struct Change {
    pub package: String,
    pub release_as: Bump,
    pub deploy: Vec<String>,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Change {
    pub package: String,
    pub release_as: Bump,
    pub deploy: Vec<String>,
}

/// Initialize the changes file. If the file does not exist, it will create it with the default message.
/// If the file exists, it will return the content of the file.
pub fn init_changes(
    cwd: Option<String>,
    change_options: &Option<ChangesOptions>,
) -> ChangesFileData {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();
        return changes;
    } else {
        let message = match &change_options {
            Some(options) => match &options.message {
                Some(msg) => msg.to_string(),
                None => String::from("chore(release): release new version"),
            },
            None => String::from("chore(release): release new version"),
        };

        let username = match &change_options {
            Some(options) => match &options.git_user_name {
                Some(name) => name.to_string(),
                None => String::from("Git Bot"),
            },
            None => String::from("Git Bot"),
        };

        let email = match &change_options {
            Some(options) => match &options.git_user_email {
                Some(email) => email.to_string(),
                None => String::from("git.bot@domain.com"),
            },
            None => String::from("git.bot@domain.com"),
        };

        let changes = ChangesFileData {
            message: Some(message),
            git_user_name: Some(username),
            git_user_email: Some(email),
            changes: ChangesData::new(),
        };

        let changes_file = File::create(changes_path).unwrap();
        let changes_writer = BufWriter::new(changes_file);

        serde_json::to_writer_pretty(changes_writer, &changes).unwrap();

        return changes;
    }
}

/// Add a change to the changes file in the root of the project.
pub fn add_change(change: &Change, cwd: Option<String>) -> bool {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let mut changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        let current_branch = git_current_branch(Some(root.to_string()));

        let branch = match current_branch {
            Some(branch) => branch,
            None => String::from("main"),
        };

        if changes.changes.contains_key(&branch) {
            let branch_changes = changes.changes.get_mut(&branch).unwrap();

            let pkg_already_added = branch_changes
                .iter()
                .any(|branch_change| branch_change.package.as_str() == change.package.as_str());

            if !pkg_already_added {
                branch_changes.push(Change {
                    package: change.package.to_string(),
                    release_as: change.release_as,
                    deploy: change.deploy.to_vec(),
                });
            }
        } else {
            changes.changes.insert(
                branch,
                vec![Change {
                    package: change.package.to_string(),
                    release_as: change.release_as,
                    deploy: change.deploy.to_vec(),
                }],
            );
        }

        let changes_file = File::create(changes_path).unwrap();
        let changes_writer = BufWriter::new(changes_file);

        serde_json::to_writer_pretty(changes_writer, &changes).unwrap();

        return true;
    }

    false
}

/// Remove a change from the changes file in the root of the project.
pub fn remove_change(branch_name: String, cwd: Option<String>) -> bool {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let mut changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        if changes.changes.contains_key(&branch_name) {
            changes.changes.remove(&branch_name);

            let changes_file = File::create(changes_path).unwrap();
            let changes_writer = BufWriter::new(changes_file);

            serde_json::to_writer_pretty(changes_writer, &changes).unwrap();

            return true;
        }
    }

    false
}

/// Get all changes from the changes file in the root of the project.
pub fn get_changes(cwd: Option<String>) -> Changes {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        return Changes {
            changes: changes.changes,
        };
    }

    Changes {
        changes: ChangesData::new(),
    }
}

/// Get all changes for a specific branch from the changes file in the root of the project.
pub fn get_change(branch: String, cwd: Option<String>) -> Vec<Change> {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        if changes.changes.contains_key(&branch) {
            return changes.changes.get(&branch).unwrap().to_vec();
        } else {
            return vec![];
        }
    }

    vec![]
}

pub fn get_package_change(package_name: String, branch: String, cwd: Option<String>) -> Option<Change> {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        if changes.changes.contains_key(&branch) {
            let branch_changes = changes.changes.get(&branch).unwrap();

            let package_change = branch_changes
                .iter()
                .find(|change| change.package == package_name);

            if let Some(change) = package_change {
                return Some(change.clone());
            }

            return None;
        }

        return None;
    }

    None
}

/// Check if a change exists in the changes file in the root of the project.
pub fn change_exist(branch: String, packages_name: Vec<String>, cwd: Option<String>) -> bool {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let changes: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        if changes.changes.contains_key(&branch) {
            let branch_changes = changes.changes.get(&branch).unwrap();

            let existing_packages_changes = branch_changes
                .iter()
                .map(|change| change.package.to_string())
                .collect::<Vec<String>>();

            let package_names_diff = packages_name
                .iter()
                .filter_map(|p| {
                    if existing_packages_changes.contains(&p) {
                        None
                    } else {
                        Some(p.to_string())
                    }
                })
                .collect::<Vec<String>>();

            match package_names_diff.len() {
                0 => return true,
                _ => return false,
            };
        }
    }

    false
}

/// Check if a changes file exists in the root of the project.
pub fn changes_file_exist(cwd: Option<String>) -> bool {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    changes_path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manager::PackageManager;
    use crate::paths::get_project_root_path;
    use crate::utils::create_test_monorepo;
    use std::fs::remove_dir_all;

    #[test]
    fn test_init_changes() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let changes_data_file = init_changes(Some(root.to_string()), &None);
        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));

        assert_eq!(changes_data_file.message.is_some(), true);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_add_change() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        let result = add_change(&change, Some(root.to_string()));

        assert_eq!(result, true);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_duplicate_add_change() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change, Some(root.to_string()));
        add_change(&change, Some(root.to_string()));

        let changes = get_changes(Some(root.to_string()));
        let length = changes.changes["main"].len();

        assert_eq!(length, 1);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_remove_change() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change, Some(root.to_string()));

        let result = remove_change(String::from("main"), Some(root.to_string()));

        assert_eq!(result, true);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_changes() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change, Some(root.to_string()));

        let changes = get_changes(Some(root.to_string()));

        assert_eq!(changes.changes.contains_key(&String::from("main")), true);
        assert_eq!(changes.changes.get(&String::from("main")).unwrap().len(), 1);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_change() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change, Some(root.to_string()));

        let changes = get_change(String::from("main"), Some(root.to_string()));

        assert_eq!(changes.len(), 1);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_change_exist() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change, Some(root.to_string()));

        let result = change_exist(
            String::from("main"),
            vec!["test-package".to_string()],
            Some(root.to_string()),
        );

        assert_eq!(result, true);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_multiple_change_exist() {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm).unwrap();
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf())).unwrap();

        let ref root = project_root.to_string();

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

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change_package_a, Some(root.to_string()));
        add_change(&change_package_b, Some(root.to_string()));

        let result = change_exist(
            String::from("main"),
            vec![
                "@scope/package-a".to_string(),
                "@scope/package-b".to_string(),
            ],
            Some(root.to_string()),
        );

        assert_eq!(result, true);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir).unwrap();
    }

    #[test]
    fn test_change_exist_with_new_package() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let change = Change {
            package: String::from("test-package"),
            release_as: Bump::Major,
            deploy: vec![String::from("production")],
        };

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        add_change(&change, Some(root.to_string()));

        let result = change_exist(
            String::from("main"),
            vec!["test-package".to_string(), "@scope/package-a".to_string()],
            Some(root.to_string()),
        );

        assert_eq!(result, false);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_change_exist_with_empty_packages() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        init_changes(Some(root.to_string()), &None);

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));

        let result = change_exist(
            String::from("main"),
            vec!["test-package".to_string(), "@scope/package-a".to_string()],
            Some(root.to_string()),
        );

        assert_eq!(result, false);
        assert_eq!(changes_path.is_file(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_changes_file_exist() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let ref changes_path = monorepo_dir.join(String::from(".changes.json"));
        let result = changes_file_exist(Some(root.to_string()));

        assert_eq!(result, false);
        assert_eq!(changes_path.is_file(), false);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
