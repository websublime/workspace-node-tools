use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::manager::{detect_package_manager, CorePackageManager};
use crate::paths::get_project_root_path;

struct WorkspaceConfig {
    package_manager: CorePackageManager,
    workspace_root: PathBuf,
    changes_config: HashMap<String, String>,
    cliff_config: String,
}

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

fn get_changes_config(root: &PathBuf) -> HashMap<String, String> {
    let default_changes_config = HashMap::from([
        ("message".to_string(), "chore(release): |---| release new version".to_string()),
        ("git_user_name".to_string(), "github-actions[bot]".to_string()),
        ("git_user_email".to_string(), "github-actions[bot]@users.noreply.git.com".to_string()),
    ]);

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_file = File::open(changes_path).unwrap();
        let changes_reader = BufReader::new(changes_file);

        let changes_config: ChangesFileData = serde_json::from_reader(changes_reader).unwrap();

        HashMap::from([
            ("message".to_string(), changes_config.message.unwrap()),
            ("git_user_name".to_string(), changes_config.git_user_name.unwrap()),
            ("git_user_email".to_string(), changes_config.git_user_email.unwrap()),
        ])
    } else {
        default_changes_config
    }
}

fn get_cliff_config(root: &PathBuf) -> String {
    "cliff.toml".to_string()
}

fn get_workspace_root(cwd: Option<PathBuf>) -> PathBuf {
    let ref root = match cwd {
        Some(ref dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };
    PathBuf::from(root)
}

pub fn get_workspace_config(cwd: Option<PathBuf>) {
    let ref root = get_workspace_root(cwd);
    let changes = get_changes_config(root);
    dbg!(&changes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::MonorepoWorkspace;

    #[test]
    fn test_get_workspace_config() -> Result<(), std::io::Error> {
        let ref monorepo = MonorepoWorkspace::new();
        let root = monorepo.get_monorepo_root().to_path_buf();
        monorepo.create_repository()?;

        dbg!(monorepo);

        get_workspace_config(Some(root.clone()));

        monorepo.delete_repository();

        Ok(())
    }
}
