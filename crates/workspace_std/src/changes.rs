//use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use crate::{
    config::{get_workspace_config, WorkspaceConfig},
    git::Repository,
};

type ChangesData = BTreeMap<String, ChangeMeta>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Change {
    pub package: String,
    pub release_as: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeMeta {
    pub deploy: Vec<String>,
    pub pkgs: Vec<Change>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Changes {
    pub changes: ChangesData,
    root: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangesConfig {
    pub message: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
    pub changes: ChangesData,
}

impl From<WorkspaceConfig> for Changes {
    fn from(config: WorkspaceConfig) -> Self {
        Changes { root: config.workspace_root.clone(), changes: ChangesData::new() }
    }
}

impl From<&WorkspaceConfig> for Changes {
    fn from(config: &WorkspaceConfig) -> Self {
        Changes { root: config.workspace_root.clone(), changes: ChangesData::new() }
    }
}

impl From<&PathBuf> for Changes {
    fn from(root: &PathBuf) -> Self {
        Changes { root: root.clone(), changes: ChangesData::new() }
    }
}

impl From<PathBuf> for Changes {
    fn from(root: PathBuf) -> Self {
        Changes { root, changes: ChangesData::new() }
    }
}

impl Changes {
    pub fn new(root: &Path) -> Self {
        Changes { root: root.to_path_buf(), changes: ChangesData::new() }
    }

    pub fn init(&self) -> ChangesConfig {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");
            return changes;
        }

        let config = get_workspace_config(Some(self.root.clone()));
        let message = config.changes_config.get("message").expect("Failed to get message changes");
        let git_user_name = config
            .changes_config
            .get("git_user_name")
            .expect("Failed to get git_user_name changes");
        let git_user_email = config
            .changes_config
            .get("git_user_email")
            .expect("Failed to get git_user_email changes");

        let changes = ChangesConfig {
            message: Some(message.to_string()),
            git_user_name: Some(git_user_name.to_string()),
            git_user_email: Some(git_user_email.to_string()),
            changes: ChangesData::new(),
        };

        let changes_file = File::create(changes_path).expect("Failed to create changes file");
        let changes_writer = BufWriter::new(changes_file);

        serde_json::to_writer_pretty(changes_writer, &changes)
            .expect("Failed to write changes file");

        changes
    }

    pub fn add(&self, change: &Change, deploy_envs: Option<Vec<String>>) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let mut changes_config: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");
            let current_branch = Repository::new(&self.root)
                .get_current_branch()
                .expect("Failed to get current branch");

            let branch = match current_branch {
                Some(branch) => branch,
                None => String::from("main"),
            };

            let envs = deploy_envs.unwrap_or_default();

            changes_config.changes.entry(branch).and_modify(|entry| {
                let pkg_exist = entry.pkgs.iter().any(|pkg| pkg.package == change.package);

                if !pkg_exist {
                    entry.deploy.extend(envs);
                    entry.pkgs.push(change.clone());
                }
            });

            return true;
        }

        false
    }

    /*pub fn remove(&self, branch_name: &str) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let mut changes: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(branch_name) {
                changes.changes.remove(branch_name);

                let changes_file =
                    File::create(changes_path).expect("Failed to create changes file");
                let changes_writer = BufWriter::new(changes_file);

                serde_json::to_writer_pretty(changes_writer, &changes)
                    .expect("Failed to write changes file");

                return true;
            }
        }

        false
    }

    pub fn changes(&self) -> ChangesData {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            return changes.changes;
        }

        ChangesData::new()
    }

    pub fn change_by_branch(&self, branch: &str) -> Vec<Change> {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(branch) {
                let branch_changes = changes.changes.get(branch);

                if branch_changes.is_none() {
                    return vec![];
                }

                return branch_changes.unwrap().clone();
            }

            return vec![];
        }

        vec![]
    }

    pub fn change_by_package(&self, package_name: &str, branch: &str) -> Option<Change> {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(branch) {
                let branch_changes =
                    changes.changes.get(branch).expect("Failed to get branch changes");

                let package_change =
                    branch_changes.iter().find(|change| change.package == package_name);

                if let Some(change) = package_change {
                    return Some(change.clone());
                }

                return None;
            }

            return None;
        }

        None
    }

    pub fn exist(&self, branch: &str, packages_name: &[String]) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesConfig =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(branch) {
                let branch_changes =
                    changes.changes.get(branch).expect("Failed to get branch changes");

                let existing_packages_changes = branch_changes
                    .iter()
                    .map(|change| change.package.to_string())
                    .collect::<Vec<String>>();

                let package_names_diff = packages_name
                    .iter()
                    .filter_map(|p| {
                        if existing_packages_changes.contains(p) {
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

    pub fn file_exist(&self) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let changes_path = &root_path.join(String::from(".changes.json"));

        changes_path.exists()
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::CorePackageManager;
    use crate::test::MonorepoWorkspace;

    #[test]
    fn test_init_changes() -> Result<(), std::io::Error> {
        let monorepo = MonorepoWorkspace::new();
        let root = monorepo.get_monorepo_root().clone();
        monorepo.create_repository(&CorePackageManager::Pnpm)?;

        let changes = Changes::new(root.as_path());
        let changes_config = changes.init();

        assert_eq!(
            changes_config.message,
            Some("chore(release): |---| release new version".to_string())
        );

        monorepo.delete_repository();

        Ok(())
    }
}
