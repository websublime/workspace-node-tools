#[cfg(test)]
use git2::{Commit, Repository, RepositoryInitOptions};

#[cfg(test)]
use std::{
    env::temp_dir,
    fs::{remove_dir_all, OpenOptions},
    io::BufWriter,
    path::PathBuf,
};

/// Strips the trailing newline from a string.
pub fn strip_trailing_newline(input: &String) -> String {
    input.strip_suffix("\r\n").or(input.strip_suffix("\n")).unwrap_or(input).trim().to_string()
}

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

    pub fn create_repository(&self) -> Result<Repository, Box<dyn std::error::Error>> {
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");

        let repository = Repository::init_opts(&self.root, &opts)?;

        let mut config = repository.config()?;
        config.set_str("user.name", "Sublime Machine")?;
        config.set_str("user.email", "machine@websublime.dev")?;

        Ok(repository)
    }

    pub fn create_monorepo_package_json(
        &self,
        repo: &Repository,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let object_id = repo.head()?.target().unwrap();
        let target = repo.find_commit(object_id)?;

        repo.branch("feature/init", &target, true)?;
        let index_file = repo.index()?;

        let monorepo_root_json = r#"
      {
          "name": "root",
          "workspaces": [
              "packages/package-foo",
              "packages/package-bar",
              "packages/package-baz",
              "packages/package-charlie"
              "packages/package-major"
              "packages/package-tom"
          ]
      }"#;
        let monorepo_package_json = &self.root.join("package.json");

        let package_root_json = serde_json::from_str::<serde_json::Value>(monorepo_root_json)?;
        let root_package_json =
            OpenOptions::new().write(true).create(true).open(monorepo_package_json)?;
        let writer = BufWriter::new(root_package_json);
        serde_json::to_writer_pretty(writer, &package_root_json)?;

        Ok(())
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
    fn git_root_project() -> Result<(), Box<dyn std::error::Error>> {
        let ref repo = MonorepoWorkspace::new();

        dbg!(repo);

        repo.create_repository()?;
        repo.delete_repository();

        Ok(())
    }
}
