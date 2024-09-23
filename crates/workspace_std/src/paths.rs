use std::{
    env,
    fs::canonicalize,
    path::{Path, PathBuf},
};

use git2::Repository;

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

    let _git_root_dir = walk_reverse_dir(&current_path);
    get_git_root_dir(&current_path);

    Some(String::from("root"))
}

fn get_git_root_dir(dir: &Path) -> Option<String> {
    let repo = Repository::open(dir).ok()?;
    let _top_level = repo.revparse("show-toplevel").ok()?;

    Some(String::from("toplevel"))
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
    //use super::*;

    #[test]
    fn git_root_project() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
