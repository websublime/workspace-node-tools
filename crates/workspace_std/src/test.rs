#[cfg(test)]
use std::{env::temp_dir, fs::remove_dir_all, path::PathBuf};

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MonorepoWorkspace {
    root: PathBuf,
}

#[cfg(test)]
impl MonorepoWorkspace {
    pub fn new() -> Self {
        let temp_dir = temp_dir();
        let monorepo_root_dir = temp_dir.join("monorepo-workspace");

        Self { root: monorepo_root_dir }
    }

    pub fn delete_repository(&self) -> bool {
        remove_dir_all(&self.root).is_ok()
    }

    pub fn get_monorepo_root(&self) -> &PathBuf {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_root_project() -> Result<(), std::io::Error> {
        let ref monorepo = MonorepoWorkspace::new();

        dbg!(monorepo);

        Ok(())
    }
}
