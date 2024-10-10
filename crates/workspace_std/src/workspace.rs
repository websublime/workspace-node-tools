use std::{
    fs::{canonicalize, File},
    io::BufReader,
    path::{Path, PathBuf},
};
use wax::{CandidatePath, Glob, Pattern};

use crate::{
    config::{get_workspace_config, WorkspaceConfig},
    manager::CorePackageManager,
    package::PackageJson,
};

pub struct Workspace {
    pub config: WorkspaceConfig,
}

impl From<&str> for Workspace {
    fn from(root: &str) -> Self {
        let path_buff = PathBuf::from(root);
        let canonic_path = canonicalize(Path::new(path_buff.as_os_str())).expect("Invalid path");
        let config = get_workspace_config(Some(canonic_path));
        Workspace { config }
    }
}

impl From<WorkspaceConfig> for Workspace {
    fn from(config: WorkspaceConfig) -> Self {
        Workspace { config }
    }
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        let config = get_workspace_config(Some(root));
        Workspace { config }
    }

    pub fn get_packages(&self) {
        let manager = self.config.package_manager;

        match manager {
            CorePackageManager::Npm | CorePackageManager::Yarn => {
                self.get_packages_from_npm();
            }
            CorePackageManager::Bun => {
                todo!("Implement Bun package manager")
            }
            CorePackageManager::Pnpm => {
                todo!("Implement Pnpm package manager")
            }
        }
    }

    fn get_root_package_json(&self) -> PackageJson {
        let package_json_path = self.config.workspace_root.join("package.json");

        let package_json_file = File::open(package_json_path.as_path()).expect("File not found");
        let package_json_buffer = BufReader::new(package_json_file);

        serde_json::from_reader(package_json_buffer).expect("Error parsing package.json")
    }

    #[allow(clippy::needless_borrows_for_generic_args)]
    fn get_packages_from_npm(&self) {
        let path = self.config.workspace_root.as_path();
        let PackageJson { workspaces, .. } = self.get_root_package_json();
        let mut workspaces = workspaces.unwrap_or_default();

        let globs = workspaces
            .iter_mut()
            .map(|workspace| {
                if workspace.ends_with("/*") {
                    workspace.push_str("*/package.json");
                    Glob::new(workspace).expect("Error parsing glob")
                } else {
                    workspace.push_str("/package.json");
                    Glob::new(workspace).expect("Error parsing glob")
                }
            })
            .collect::<Vec<Glob>>();

        let patterns = wax::any(globs).expect("Error creating patterns");
        let glob = Glob::new("**/package.json").expect("Error parsing glob");

        for entry in glob
            .walk(self.config.workspace_root.as_path())
            .not([
                "**/node_modules/**",
                "**/src/**",
                "**/dist/**",
                "**/tests/**",
                "**/__tests__/**",
            ])
            .expect("Error walking glob")
        {
            let entry = entry.expect("Error reading entry");
            let _rel_path = entry
                .path()
                .strip_prefix(&path)
                .expect("Error getting entry path")
                .display()
                .to_string();
            let entry_path = entry.path().strip_prefix(&path).expect("Error getting entry path");

            if patterns.is_match(CandidatePath::from(entry_path)) {
                let package_json_file = File::open(&entry.path()).expect("File not found");
                let package_json_reader = BufReader::new(package_json_file);
                let pkg_json: PackageJson = serde_json::from_reader(package_json_reader)
                    .expect("Failed to parse package json file");

                let mut package_dependencies = vec![];

                let dependencies = pkg_json.dependencies.unwrap_or_default();
                let dev_dependencies = pkg_json.dev_dependencies.unwrap_or_default();
                let peer_dependencies = pkg_json.peer_dependencies.unwrap_or_default();
                let optional_dependencies = pkg_json.optional_dependencies.unwrap_or_default();

                if dependencies.is_object() {
                    package_dependencies.push(dependencies);
                }

                if dev_dependencies.is_object() {
                    package_dependencies.push(dev_dependencies);
                }

                if peer_dependencies.is_object() {
                    package_dependencies.push(peer_dependencies);
                }

                if optional_dependencies.is_object() {
                    package_dependencies.push(optional_dependencies);
                }

                //let dependencies = dependencies.into_iter().flatten().collect::<Vec<_>>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::CorePackageManager;
    use crate::test::MonorepoWorkspace;

    #[test]
    fn test_workspace() -> Result<(), std::io::Error> {
        let monorepo = MonorepoWorkspace::new();
        let root = monorepo.get_monorepo_root().clone();
        monorepo.create_workspace(&CorePackageManager::Npm)?;

        let workspace = Workspace::new(root);
        workspace.get_packages();

        monorepo.delete_repository();

        Ok(())
    }
}
