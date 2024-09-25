use crate::errors::GitError;
use crate::types::GitResult;
use crate::utils::strip_trailing_newline;

use regex::Regex;
use std::ffi::OsStr;
use std::io::Write;
use std::{
    env::temp_dir,
    fs::{remove_file, File},
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

    pub fn config(&self, username: &String, email: &String) -> GitResult<bool> {
        execute_git(
            &self.location,
            &["config", "user.name", username.as_str(), "user.email", email.as_str()],
            |_, output| Ok(output.status.success()),
        )
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

        let mut args = vec!["--no-pager", "log", log_format.as_str(), "--date=rfc2822"];

        /*if since.is_some() {
            let log_since = "{}..".to_owned() + since.unwrap().as_str();
            args.push(log_since.as_str());
        }

        if relative.is_some() {
            args.push("--");
            args.push(relative.unwrap().as_str());
        }*/

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

fn execute_git<P, I, F, S, R>(path: P, args: I, process: F) -> GitResult<R>
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

    #[test]
    fn test_repo() -> Result<(), std::io::Error> {
        let temp_dir = temp_dir();
        let monorepo_root_dir = temp_dir.join("monorepo-workspace");

        create_dir(&monorepo_root_dir)?;

        #[cfg(not(windows))]
        std::fs::set_permissions(&monorepo_root_dir, std::fs::Permissions::from_mode(0o777))?;

        let repo = Repository::new(monorepo_root_dir.as_path());

        dbg!(repo);

        remove_dir_all(&monorepo_root_dir)?;

        Ok(())
    }
}
