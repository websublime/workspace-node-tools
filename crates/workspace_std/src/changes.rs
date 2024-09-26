use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::WorkspaceConfig;

type ChangesData = BTreeMap<String, Vec<Change>>;

#[derive(Deserialize, Serialize)]
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

pub struct Changes {
    changes: ChangesData,
    root: PathBuf,
}

impl From<WorkspaceConfig> for Changes {
    fn from(config: WorkspaceConfig) -> Self {
        Changes { root: config.workspace_root.to_path_buf(), changes: BTreeMap::new() }
    }
}
