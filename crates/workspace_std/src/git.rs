//! This module provides a simple interface to interact with Git repositories.
//! To use this module import it like this:
//! ```rust
//! use workspace_std::git::Repository;
//! ```
use crate::errors::GitError;
use crate::types::GitResult;
use crate::utils::strip_trailing_newline;

use regex::Regex;
use std::{
    collections::HashMap,
    env::temp_dir,
    ffi::OsStr,
    fs::{remove_file, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    str,
};

#[derive(Debug, Clone)]
pub struct Repository {
    location: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RepositoryCommit {
    hash: String,
    author_name: String,
    author_email: String,
    author_date: String,
    message: String,
}

#[derive(Debug, Clone)]
pub struct RepositoryRemoteTags {
    tag: String,
    hash: String,
}

impl Repository {
    pub fn new(location: &Path) -> Self {
        Self { location: location.to_path_buf() }
    }

    pub fn get_repo_path(&self) -> &Path {
        &self.location
    }

    pub fn init(
        &self,
        initial_branch: &String,
        username: &String,
        email: &String,
    ) -> GitResult<bool> {
        let init = execute_git(
            &self.location,
            &["init", "--initial-branch", initial_branch.as_str()],
            |_, output| Ok(output.status.success()),
        );
        let config = self.config(username, email);

        Ok(init.is_ok() && config.is_ok())
    }

    pub fn is_vcs(&self) -> GitResult<bool> {
        execute_git(&self.location, &["rev-parse", "--is-inside-work-tree"], |stdout, _| {
            Ok(stdout.trim() == "true")
        })
    }

    pub fn config(&self, username: &String, email: &String) -> GitResult<bool> {
        let user_config = execute_git(
            &self.location,
            &["config", "user.name", username.as_str()],
            |_, output| Ok(output.status.success()),
        );

        let email_config =
            execute_git(&self.location, &["config", "user.email", email.as_str()], |_, output| {
                Ok(output.status.success())
            });

        Ok(user_config.is_ok() && email_config.is_ok())
    }

    pub fn add_all(&self) -> GitResult<bool> {
        execute_git(&self.location, &["add", "."], |_, output| Ok(output.status.success()))
    }

    pub fn add(&self, path: &Path) -> GitResult<bool> {
        if path.to_str().is_some() {
            execute_git(&self.location, &["add", path.to_str().unwrap()], |_, output| {
                Ok(output.status.success())
            })
        } else {
            Ok(false)
        }
    }

    pub fn fetch_all(&self, fetch_tags: Option<bool>) -> GitResult<bool> {
        let mut args = vec!["fetch", "origin"];

        if fetch_tags.unwrap_or(false) {
            args.push("--tags");
            args.push("--force");
        }

        execute_git(&self.location, &args, |_, output| Ok(output.status.success()))
    }

    pub fn get_diverged_commit(&self, sha: &String) -> GitResult<String> {
        execute_git(&self.location, &["merge-base", sha.as_str(), "HEAD"], |stdout, _| {
            Ok(stdout.to_string())
        })
    }

    pub fn get_current_sha(&self) -> GitResult<String> {
        execute_git(&self.location, &["rev-parse", "--short", "HEAD"], |stdout, _| {
            Ok(stdout.to_string())
        })
    }

    pub fn get_previous_sha(&self) -> GitResult<String> {
        execute_git(&self.location, &["rev-parse", "--short", "HEAD~1"], |stdout, _| {
            Ok(stdout.to_string())
        })
    }

    pub fn get_first_sha(&self, branch: Option<String>) -> GitResult<String> {
        let branch = match branch {
            Some(branch) => branch,
            None => String::from("main"),
        };

        execute_git(
            &self.location,
            &[
                "",
                format!("{}..HEAD", branch).as_str(),
                "--online",
                "--pretty=format:%h",
                "|",
                "tail",
                "-1",
            ],
            |stdout, _| Ok(stdout.to_string()),
        )
    }

    pub fn is_workdir_unclean(&self) -> GitResult<bool> {
        execute_git(&self.location, &["status", "--porcelain"], |stdout, _| Ok(!stdout.is_empty()))
    }

    pub fn get_current_branch(&self) -> GitResult<Option<String>> {
        execute_git(&self.location, &["rev-parse", "--abbrev-ref", "HEAD"], |stdout, _| {
            if stdout.is_empty() {
                Ok(None)
            } else {
                Ok(Some(stdout.to_string()))
            }
        })
    }

    pub fn get_branch_from_commit(&self, sha: &String) -> GitResult<Option<String>> {
        execute_git(
            &self.location,
            &[
                "--no-pager",
                "branch",
                "--no-color",
                "--no-column",
                "--format",
                r#""%(refname:lstrip=2)""#,
                "--contains",
                sha.as_str(),
            ],
            |stdout, _| {
                if stdout.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(stdout.to_string()))
                }
            },
        )
    }

    pub fn tag(&self, tag: &String, message: Option<String>) -> GitResult<bool> {
        let msg = message.unwrap_or(tag.to_string());

        execute_git(
            &self.location,
            &["tag", "-a", tag.as_str(), "-m", msg.as_str()],
            |_, output| Ok(output.status.success()),
        )
    }

    pub fn push(&self, follow_tags: Option<bool>) -> GitResult<bool> {
        let mut args = vec!["push", "--no-verify"];

        if follow_tags.unwrap_or(false) {
            args.push("--follow-tags");
        }

        execute_git(&self.location, &args, |_, output| Ok(output.status.success()))
    }

    pub fn commit(
        &self,
        message: &String,
        body: Option<String>,
        footer: Option<String>,
    ) -> GitResult<bool> {
        let mut msg = message.to_string();

        if body.is_some() {
            msg.push_str("\n\n");
            msg.push_str(body.unwrap().as_str());
        }

        if footer.is_some() {
            msg.push_str("\n\n");
            msg.push_str(footer.unwrap().as_str());
        }

        let temp_dir = temp_dir();
        let temp_file_path = temp_dir.join("commit_message.txt");

        let mut file = File::create(&temp_file_path).expect("Failed to creat commit file");
        file.write_all(message.as_bytes()).expect("Failed to write commit message");

        let file_path = temp_file_path.as_path();

        execute_git(
            &self.location,
            &[
                "commit",
                "-F",
                file_path.to_str().expect("Failed to retrieve file_path"),
                "--no-verify",
            ],
            |_, output| {
                remove_file(file_path).expect("Commit file not deleted");

                Ok(output.status.success())
            },
        )
    }

    pub fn get_all_files_changed_since_sha(&self, sha: &String) -> GitResult<Vec<String>> {
        execute_git(
            &self.location,
            &["--no-pager", "diff", "--name-only", sha.as_str(), "HEAD"],
            |stdout, output| {
                if !output.status.success() {
                    return Ok(vec![]);
                }

                Ok(stdout
                    .split("\n")
                    .filter(|item| !item.trim().is_empty())
                    .map(|item| self.location.join(item))
                    .filter(|item| item.exists())
                    .map(|item| {
                        item.to_str().expect("Failed to convert path to string").to_string()
                    })
                    .collect::<Vec<String>>())
            },
        )
    }

    pub fn get_commits_since(
        &self,
        since: Option<String>,
        relative: Option<String>,
    ) -> GitResult<Vec<RepositoryCommit>> {
        const DELIMITER: &str = r#"#=#"#;
        const BREAK_LINE: &str = r#"#+#"#;

        let log_format = format!(
            "--format={}%H{}%an{}%ae{}%ad{}%B{}",
            DELIMITER, DELIMITER, DELIMITER, DELIMITER, DELIMITER, BREAK_LINE
        );

        let mut args = vec![
            "--no-pager".to_string(),
            "log".to_string(),
            log_format,
            "--date=rfc2822".to_string(),
        ];

        if let Some(since) = since {
            args.push(format!("{}..", since));
        }

        if let Some(relative) = relative {
            args.push("--".to_string());
            args.push(relative);
        }

        execute_git(&self.location, &args, |stdout, output| {
            if !output.status.success() {
                return Ok(vec![]);
            }

            Ok(stdout
                .split(BREAK_LINE)
                .filter(|item| !item.trim().is_empty())
                .map(|item| {
                    let item_trimmed = item.trim();
                    let items = item_trimmed.split(DELIMITER).collect::<Vec<&str>>();

                    RepositoryCommit {
                        hash: items.get(1).unwrap().to_string(),
                        author_name: items.get(2).unwrap().to_string(),
                        author_email: items.get(3).unwrap().to_string(),
                        author_date: items.get(4).unwrap().to_string(),
                        message: items.get(5).unwrap().to_string(),
                    }
                })
                .collect::<Vec<RepositoryCommit>>())
        })
    }

    pub fn get_remote_or_local_tags(
        &self,
        local: Option<bool>,
    ) -> GitResult<Vec<RepositoryRemoteTags>> {
        let mut args = vec![];

        match local {
            Some(true) => {
                args.push("show-ref");
                args.push("--tags");
            }
            Some(false) => {
                args.push("ls-remote");
                args.push("--tags");
                args.push("origin");
            }
            None => {
                args.push("ls-remote");
                args.push("--tags");
                args.push("origin");
            }
        }

        execute_git(&self.location, &args, |stdout, output| {
            if !output.status.success() {
                return Ok(vec![]);
            }

            #[cfg(windows)]
            const LINE_ENDING: &'static str = "\r\n";
            #[cfg(not(windows))]
            const LINE_ENDING: &'static str = "\n";

            Ok(stdout
                .trim()
                .split(LINE_ENDING)
                .filter(|tags| !tags.trim().is_empty())
                .map(|tags| {
                    let hash_tags = Regex::new(r"\s+").unwrap().split(tags).collect::<Vec<&str>>();

                    RepositoryRemoteTags {
                        hash: hash_tags.get(0).unwrap().to_string(),
                        tag: hash_tags.get(1).unwrap().to_string(),
                    }
                })
                .collect::<Vec<RepositoryRemoteTags>>())
        })
    }

    pub fn get_all_files_changed_since_branch(
        &self,
        packages_paths: &Vec<String>,
        branch: &String,
    ) -> Vec<String> {
        let mut all_files = vec![];

        packages_paths.iter().for_each(|item| {
            let files = self
                .get_all_files_changed_since_sha(&branch.to_string())
                .expect("Failed to retrieve files changed since branch");

            let pkg_files = files
                .iter()
                .filter(|file| file.starts_with(item.as_str()))
                .collect::<Vec<&String>>();

            all_files.append(
                &mut pkg_files.iter().map(|file| file.to_string()).collect::<Vec<String>>(),
            );
        });

        all_files
    }
}

impl RepositoryCommit {
    pub fn new(
        hash: String,
        author_name: String,
        author_email: String,
        author_date: String,
        message: String,
    ) -> Self {
        Self { hash, author_name, author_email, author_date, message }
    }

    pub fn get_message(&self) -> &String {
        &self.message
    }

    pub fn set_message(&mut self, message: &String) {
        self.message = message.to_string();
    }

    pub fn get_author_name(&self) -> &String {
        &self.author_name
    }

    pub fn set_author_name(&mut self, author_name: &String) {
        self.author_name = author_name.to_string();
    }

    pub fn get_author_email(&self) -> &String {
        &self.author_email
    }

    pub fn set_author_email(&mut self, author_email: &String) {
        self.author_email = author_email.to_string();
    }

    pub fn get_author_date(&self) -> &String {
        &self.author_date
    }

    pub fn set_author_date(&mut self, author_date: &String) {
        self.author_date = author_date.to_string();
    }

    pub fn get_hash(&self) -> &String {
        &self.hash
    }

    pub fn set_hash(&mut self, hash: &String) {
        self.hash = hash.to_string();
    }

    pub fn get_hash_map(&self) -> HashMap<String, String> {
        HashMap::from([
            ("hash".to_string(), self.hash.to_string()),
            ("author_name".to_string(), self.author_name.to_string()),
            ("author_email".to_string(), self.author_email.to_string()),
            ("author_date".to_string(), self.author_date.to_string()),
            ("message".to_string(), self.message.to_string()),
        ])
    }
}

impl RepositoryRemoteTags {
    pub fn new(hash: String, tag: String) -> Self {
        Self { hash, tag }
    }

    pub fn get_hash(&self) -> &String {
        &self.hash
    }

    pub fn set_hash(&mut self, hash: &String) {
        self.hash = hash.to_string();
    }

    pub fn get_tag(&self) -> &String {
        &self.tag
    }

    pub fn set_tag(&mut self, tag: &String) {
        self.tag = tag.to_string();
    }

    pub fn get_hash_map(&self) -> HashMap<String, String> {
        HashMap::from([
            ("hash".to_string(), self.hash.to_string()),
            ("tag".to_string(), self.tag.to_string()),
        ])
    }
}

pub fn execute_git<P, I, F, S, R>(path: P, args: I, process: F) -> GitResult<R>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    F: Fn(&str, &Output) -> GitResult<R>,
{
    let output = Command::new("git").current_dir(path).args(args).output();

    output.map_err(|_| GitError::Execution).and_then(|output| {
        if output.status.success() {
            if let Ok(message) = str::from_utf8(&output.stdout) {
                process(strip_trailing_newline(&message.to_string()).as_str(), &output)
            } else {
                Err(GitError::Execution)
            }
        } else {
            if let Ok(message) = str::from_utf8(&output.stdout) {
                if let Ok(err) = str::from_utf8(&output.stderr) {
                    Err(GitError::GitError { stdout: message.to_string(), stderr: err.to_string() })
                } else {
                    Err(GitError::Execution)
                }
            } else {
                Err(GitError::Execution)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, remove_dir_all};
    #[cfg(not(windows))]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn create_monorepo() -> Result<PathBuf, std::io::Error> {
        let temp_dir = temp_dir();
        let monorepo_root_dir = temp_dir.join("monorepo-workspace");

        if monorepo_root_dir.exists() {
            remove_dir_all(&monorepo_root_dir)?;
        }

        create_dir(&monorepo_root_dir)?;

        #[cfg(not(windows))]
        std::fs::set_permissions(&monorepo_root_dir, std::fs::Permissions::from_mode(0o777))?;

        Ok(monorepo_root_dir)
    }

    #[test]
    fn test_create_repo() -> Result<(), std::io::Error> {
        let monorepo_root_dir = create_monorepo()?;
        let repo = Repository::new(&monorepo_root_dir);
        let result = repo.init(
            &"main".to_string(),
            &"Websublime Machine".to_string(),
            &"machine@websublime.com".to_string(),
        );

        assert_eq!(result.is_ok_and(|ok| ok), true);
        assert_eq!(repo.is_vcs().expect("Repo is not a vcs system"), true);

        remove_dir_all(&monorepo_root_dir)?;

        Ok(())
    }
}
