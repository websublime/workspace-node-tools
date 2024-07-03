#![allow(clippy::unwrap_or_default)]
#![allow(clippy::useless_vec)]

use std::{env, path};

pub fn get_project_root_path() -> Option<String> {
    let current_dir = env::current_dir().unwrap();
    let dir = walk_reverse_dir(current_dir.as_path()).unwrap_or_default();

    Some(dir)
}

fn walk_reverse_dir(path: &path::Path) -> Option<String> {
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

    use std::fs::{remove_file, File};
    use std::path::Path;

    fn create_agent_file(path: &Path) -> File {
        File::create(path).expect("File not created")
    }

    fn delete_agent_file(path: &Path) {
        remove_file(path).expect("File not deleted");
    }

    #[test]
    fn npm_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");

        create_agent_file(&npm_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&npm_lock);
    }

    #[test]
    fn yarn_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let yarn_lock = path.join("yarn.lock");

        create_agent_file(&yarn_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&yarn_lock);
    }

    #[test]
    fn pnpm_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let pnpm_lock = path.join("pnpm-lock.yaml");

        create_agent_file(&pnpm_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&pnpm_lock);
    }

    #[test]
    fn bun_root_project() {
        let path = std::env::current_dir().expect("Current user home directory");
        let bun_lock = path.join("bun.lockb");

        create_agent_file(&bun_lock);

        let project_root = get_project_root_path();

        assert_eq!(project_root, Some(path.to_str().unwrap().to_string()));

        delete_agent_file(&bun_lock);
    }
}
