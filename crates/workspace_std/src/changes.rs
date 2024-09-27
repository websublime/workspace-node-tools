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

type ChangesData = BTreeMap<String, Vec<Change>>;

#[derive(Clone, Deserialize, Serialize)]
pub struct Change {
    pub package: String,
    pub release_as: String,
    pub deploy: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ChangesFileData {
    pub message: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
    pub changes: ChangesData,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Changes {
    pub changes: ChangesData,
    root: PathBuf,
}

impl From<WorkspaceConfig> for Changes {
    fn from(config: WorkspaceConfig) -> Self {
        Changes { root: config.workspace_root.to_path_buf(), changes: ChangesData::new() }
    }
}

impl From<&WorkspaceConfig> for Changes {
    fn from(config: &WorkspaceConfig) -> Self {
        Changes { root: config.workspace_root.to_path_buf(), changes: ChangesData::new() }
    }
}

impl From<&PathBuf> for Changes {
    fn from(root: &PathBuf) -> Self {
        Changes { root: root.to_path_buf(), changes: ChangesData::new() }
    }
}

impl From<PathBuf> for Changes {
    fn from(root: PathBuf) -> Self {
        Changes { root, changes: ChangesData::new() }
    }
}

impl Changes {
    pub fn new(root: &PathBuf) -> Self {
        Changes { root: root.to_path_buf(), changes: ChangesData::new() }
    }

    pub fn init(&self) -> ChangesFileData {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");
            return changes;
        } else {
            let config = get_workspace_config(Some(self.root.to_path_buf()));
            let message =
                config.changes_config.get("message").expect("Failed to get message changes");
            let git_user_name = config
                .changes_config
                .get("git_user_name")
                .expect("Failed to get git_user_name changes");
            let git_user_email = config
                .changes_config
                .get("git_user_email")
                .expect("Failed to get git_user_email changes");

            let changes = ChangesFileData {
                message: Some(message.to_string()),
                git_user_name: Some(git_user_name.to_string()),
                git_user_email: Some(git_user_email.to_string()),
                changes: ChangesData::new(),
            };

            let changes_file = File::create(changes_path).expect("Failed to create changes file");
            let changes_writer = BufWriter::new(changes_file);

            serde_json::to_writer_pretty(changes_writer, &changes)
                .expect("Failed to write changes file");

            return changes;
        }
    }

    pub fn add(&self, change: &Change) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let mut changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");
            let current_branch = Repository::new(&self.root)
                .get_current_branch()
                .expect("Failed to get current branch");

            let branch = match current_branch {
                Some(branch) => branch,
                None => String::from("main"),
            };

            if changes.changes.contains_key(&branch) {
                let branch_changes =
                    changes.changes.get_mut(&branch).expect("Failed to get branch changes");

                let pkg_already_added = branch_changes
                    .iter()
                    .any(|branch_change| branch_change.package.as_str() == change.package.as_str());

                if !pkg_already_added {
                    branch_changes.push(Change {
                        package: change.package.to_string(),
                        release_as: change.release_as.to_string(),
                        deploy: change.deploy.to_vec(),
                    });
                }
            } else {
                changes.changes.insert(
                    branch,
                    vec![Change {
                        package: change.package.to_string(),
                        release_as: change.release_as.to_string(),
                        deploy: change.deploy.to_vec(),
                    }],
                );
            }

            let changes_file = File::create(changes_path).expect("Failed to create changes file");
            let changes_writer = BufWriter::new(changes_file);

            serde_json::to_writer_pretty(changes_writer, &changes)
                .expect("Failed to write changes file");

            return true;
        }

        false
    }

    pub fn remove(&self, branch_name: String) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let mut changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(&branch_name) {
                changes.changes.remove(&branch_name);

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
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            return changes.changes;
        }

        return ChangesData::new();
    }

    pub fn change_by_branch(&self, branch: String) -> Vec<Change> {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(&branch) {
                let branch_changes = changes.changes.get(&branch);

                if branch_changes.is_none() {
                    return vec![];
                }

                return branch_changes.unwrap().to_vec();
            } else {
                return vec![];
            }
        }

        vec![]
    }

    pub fn change_by_package(&self, package_name: String, branch: String) -> Option<Change> {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(&branch) {
                let branch_changes =
                    changes.changes.get(&branch).expect("Failed to get branch changes");

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

    pub fn exist(&self, branch: String, packages_name: Vec<String>) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        if changes_path.exists() {
            let changes_file = File::open(changes_path).expect("Failed to open changes file");
            let changes_reader = BufReader::new(changes_file);

            let changes: ChangesFileData =
                serde_json::from_reader(changes_reader).expect("Failed to parse changes json file");

            if changes.changes.contains_key(&branch) {
                let branch_changes =
                    changes.changes.get(&branch).expect("Failed to get branch changes");

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

    pub fn file_exist(&self) -> bool {
        let root_path = Path::new(self.root.as_os_str());
        let ref changes_path = root_path.join(String::from(".changes.json"));

        changes_path.exists()
    }
}
