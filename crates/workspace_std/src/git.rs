use crate::errors::GitError;
use crate::types::GitResult;
use crate::utils::strip_trailing_newline;

use std::ffi::OsStr;
use std::io::Write;
use std::{
    env::temp_dir,
    fs::{remove_file, File},
    path::{Path, PathBuf},
    process::Command,
    str,
};

pub struct Repository {
    location: PathBuf,
}

impl Repository {
    pub fn new(location: &Path) -> Self {
        Self { location: location.to_path_buf() }
    }

    pub fn get_repo_path(&self) -> &Path {
        &self.location
    }

    pub fn config(&self, username: &String, email: &String) -> GitResult<()> {
        execute_git(
            &self.location,
            &["config", "user.name", username.as_str(), "user.email", email.as_str()],
            |_output| Ok(()),
        )
    }

    pub fn add_all(&self) -> GitResult<bool> {
        execute_git(&self.location, &["add", "."], |output| {
            if output.is_empty() {
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    pub fn add(&self, path: &Path) -> GitResult<bool> {
        if path.to_str().is_some() {
            execute_git(&self.location, &["add", path.to_str().unwrap()], |output| {
                if output.is_empty() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        } else {
            Ok(false)
        }
    }

    pub fn fetch_all(&self, fetch_tags: Option<bool>) -> GitResult<()> {
        let mut args = vec!["fetch", "origin"];

        if fetch_tags.unwrap_or(false) {
            args.push("--tags");
            args.push("--force");
        }

        execute_git(&self.location, &args, |_output| Ok(()))
    }

    pub fn get_diverged_commit(&self, sha: &String) -> GitResult<String> {
        execute_git(&self.location, &["merge-base", sha.as_str(), "HEAD"], |output| {
            Ok(output.to_string())
        })
    }

    pub fn get_current_sha(&self) -> GitResult<String> {
        execute_git(&self.location, &["rev-parse", "--short", "HEAD"], |output| {
            Ok(output.to_string())
        })
    }

    pub fn get_previous_sha(&self) -> GitResult<String> {
        execute_git(&self.location, &["rev-parse", "--short", "HEAD~1"], |output| {
            Ok(output.to_string())
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
            |output| Ok(output.to_string()),
        )
    }

    pub fn is_workdir_unclean(&self) -> GitResult<bool> {
        execute_git(&self.location, &["status", "--porcelain"], |output| {
            if output.is_empty() {
                Ok(false)
            } else {
                Ok(true)
            }
        })
    }

    pub fn get_current_branch(&self) -> GitResult<Option<String>> {
        execute_git(&self.location, &["rev-parse", "--abbrev-ref", "HEAD"], |output| {
            if output.is_empty() {
                Ok(None)
            } else {
                Ok(Some(output.to_string()))
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
            |output| {
                if output.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(output.to_string()))
                }
            },
        )
    }

    pub fn tag(&self, tag: &String, message: Option<String>) -> GitResult<bool> {
        let msg = message.unwrap_or(tag.to_string());

        execute_git(&self.location, &["tag", "-a", tag.as_str(), "-m", msg.as_str()], |output| {
            if output.is_empty() {
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    pub fn push(&self, follow_tags: Option<bool>) -> GitResult<bool> {
        let mut args = vec!["push", "--no-verify"];

        if follow_tags.unwrap_or(false) {
            args.push("--follow-tags");
        }

        execute_git(
            &self.location,
            &args,
            |output| {
                if output.is_empty() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )
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

        let mut file = File::create(&temp_file_path).unwrap();
        file.write_all(message.as_bytes()).unwrap();

        let file_path = temp_file_path.as_path();

        execute_git(
            &self.location,
            &["commit", "-F", file_path.to_str().unwrap(), "--no-verify"],
            |output| {
                remove_file(file_path).expect("Commit file not deleted");

                if output.is_empty() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )
    }
}

fn execute_git<P, I, F, S, R>(path: P, args: I, process: F) -> GitResult<R>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    F: Fn(&str) -> GitResult<R>,
{
    let output = Command::new("git").current_dir(path).args(args).output();

    output.map_err(|_| GitError::Execution).and_then(|output| {
        if output.status.success() {
            if let Ok(message) = str::from_utf8(&output.stdout) {
                process(strip_trailing_newline(&message.to_string()).as_str())
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
