#![allow(clippy::unwrap_or_default)]
#![allow(clippy::useless_vec)]

//! #Paths module
//!
//! The `paths` module is used to get the project root path.
use execute::Execute;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Get the project root path.
pub fn get_project_root_path() -> Option<String> {
    let env_dir = env::current_dir();

    let current_dir = match env_dir {
        Ok(dir) => dir,
        _ => PathBuf::from("./"),
    };
    let current_path = current_dir.as_path();

    let git_root_dir = get_git_root_dir(&current_path);

    let project_root = match git_root_dir {
        Some(current) => current,
        None => {
            let search_root = walk_reverse_dir(&current_path);
            search_root.unwrap_or(current_path.to_str().unwrap().to_string())
        }
    };

    Some(project_root)
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
        return Some(output.trim().to_string());
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

    use std::fs::{remove_file, rename, File};
    use std::path::Path;

    fn create_agent_file(path: &Path) -> File {
        File::create(path).expect("File not created")
    }

    fn delete_agent_file(path: &Path) {
        remove_file(path).expect("File not deleted");
    }

    fn git_dir_rename(from: &Path, to: &Path) {
        rename(from, to).expect("Rename dir");
    }

    #[test]
    fn npm_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");
        let git_home = path.join(".git");
        let no_git = path.join(".no_git");

        git_dir_rename(&git_home, &no_git);
        create_agent_file(&npm_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&npm_lock);
        git_dir_rename(&no_git, &git_home);
    }

    #[test]
    fn yarn_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let yarn_lock = path.join("yarn.lock");
        let git_home = path.join(".git");
        let no_git = path.join(".no_git");

        git_dir_rename(&git_home, &no_git);
        create_agent_file(&yarn_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&yarn_lock);
        git_dir_rename(&no_git, &git_home);
    }

    #[test]
    fn pnpm_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let pnpm_lock = path.join("pnpm-lock.yaml");
        let git_home = path.join(".git");
        let no_git = path.join(".no_git");

        git_dir_rename(&git_home, &no_git);
        create_agent_file(&pnpm_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&pnpm_lock);
        git_dir_rename(&no_git, &git_home);
    }

    #[test]
    fn bun_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let bun_lock = path.join("bun.lockb");
        let git_home = path.join(".git");
        let no_git = path.join(".no_git");

        git_dir_rename(&git_home, &no_git);
        create_agent_file(&bun_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&bun_lock);
        git_dir_rename(&no_git, &git_home);
    }

    #[test]
    fn git_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));
    }
}
