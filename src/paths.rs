#![allow(clippy::all)]

//! #Paths module
//!
//! The `paths` module is used to get the project root path.
use super::utils::strip_trailing_newline;
use execute::Execute;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Get the project root path.
pub fn get_project_root_path(root: Option<PathBuf>) -> Option<String> {
    let env_dir = match root {
        Some(dir) => Ok(dir),
        None => env::current_dir(),
    };

    let current_dir = match env_dir {
        Ok(dir) => dir,
        _ => PathBuf::from("./"),
    };
    let current_path = current_dir.as_path();

    let git_root_dir = walk_reverse_dir(&current_path);

    let project_root = match git_root_dir {
        Some(current) => current,
        None => {
            let search_root = get_git_root_dir(&current_path);
            search_root.unwrap_or(current_path.to_str().unwrap().to_string())
        }
    };

    let canonic_path = &std::fs::canonicalize(Path::new(&project_root)).unwrap();
    let root = canonic_path.as_path().display().to_string();

    Some(root)
}

/// Get the git root directory.
fn get_git_root_dir(dir: &Path) -> Option<String> {
    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--show-toplevel");

    command.current_dir(dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if output.status.success() {
        let output = String::from_utf8(output.stdout).unwrap();
        return Some(strip_trailing_newline(&output));
    }

    None
}

/// Walk reverse directory to find the root project.
fn walk_reverse_dir(path: &Path) -> Option<String> {
    let current_path = path.to_path_buf();
    let map_files = vec![
        ("package-lock.json", "npm"),
        ("npm-shrinkwrap.json", "npm"),
        ("yarn.lock", "yarn"),
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lockb", "bun"),
    ];

    for (file, _) in map_files.iter() {
        let lock_file = current_path.join(file);

        if lock_file.exists() {
            return Some(current_path.to_str().unwrap().to_string());
        }
    }

    if let Some(parent) = path.parent() {
        return walk_reverse_dir(parent);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manager::PackageManager;
    use crate::utils::create_test_monorepo;
    use std::fs::{remove_dir_all, rename};
    use std::path::Path;

    fn git_dir_rename(from: &Path, to: &Path) {
        rename(from, to).expect("Rename dir");
    }

    #[test]
    fn npm_root_project() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let git_home = monorepo_dir.join(".git");
        let no_git = monorepo_dir.join(".no_git");

        git_dir_rename(&git_home, &no_git);

        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        assert_eq!(
            project_root,
            Some(monorepo_dir.to_str().unwrap().to_string())
        );

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn yarn_root_project() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Yarn)?;
        let git_home = monorepo_dir.join(".git");
        let no_git = monorepo_dir.join(".no_git");

        git_dir_rename(&git_home, &no_git);

        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        assert_eq!(
            project_root,
            Some(monorepo_dir.to_str().unwrap().to_string())
        );

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn pnpm_root_project() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Pnpm)?;
        let git_home = monorepo_dir.join(".git");
        let no_git = monorepo_dir.join(".no_git");

        git_dir_rename(&git_home, &no_git);

        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        assert_eq!(
            project_root,
            Some(monorepo_dir.to_str().unwrap().to_string())
        );

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn bun_root_project() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Bun)?;
        let git_home = monorepo_dir.join(".git");
        let no_git = monorepo_dir.join(".no_git");

        git_dir_rename(&git_home, &no_git);

        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        assert_eq!(
            project_root,
            Some(monorepo_dir.to_str().unwrap().to_string())
        );

        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn git_root_project() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        assert_eq!(project_root.is_some(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
