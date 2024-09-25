use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manager::{detect_package_manager, CorePackageManager};
use crate::paths::get_project_root_path;

struct WorkspaceConfig {
    package_manager: CorePackageManager,
    workspace_root: PathBuf,
    changes_config: HashMap<String, String>,
    cliff_config: String,
}

fn get_changes_config(root: &PathBuf) -> HashMap<String, String> {
    let default_changes_config = HashMap::from([
        ("message".to_string(), "chore(release): release new version".to_string()),
        ("git_user_name".to_string(), "github-actions[bot]".to_string()),
        ("git_user_email".to_string(), "github-actions[bot]@users.noreply.git.com".to_string()),
    ]);

    let root_path = Path::new(root);
    let ref changes_path = root_path.join(String::from(".changes.json"));

    if changes_path.exists() {
        let changes_config = std::fs::read_to_string(changes_path).unwrap();
        let changes_config: HashMap<String, String> =
            serde_json::from_str(&changes_config).unwrap();
        changes_config
    } else {
        default_changes_config
    }
}

fn get_default_cliff_config(root: &PathBuf) -> String {
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
    get_workspace_root(cwd);
}
