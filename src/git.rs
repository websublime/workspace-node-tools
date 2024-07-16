//! # Git
//!
//! This module provides a set of functions to interact with git.
use execute::Execute;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{
    env::temp_dir,
    fs::{remove_file, File},
    path::Path,
    process::{Command, Stdio},
};

use super::packages::PackageInfo;
use super::paths::get_project_root_path;
use super::utils::strip_trailing_newline;

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commit {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub author_date: String,
    pub message: String,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commit {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub author_date: String,
    pub message: String,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteTags {
    pub hash: String,
    pub tag: String,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteTags {
    pub hash: String,
    pub tag: String,
}

pub fn git_fetch_all(
    cwd: Option<String>,
    fetch_tags: Option<bool>,
) -> Result<bool, std::io::Error> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("fetch").arg("origin");

    if fetch_tags.unwrap_or(false) {
        command.arg("--tags").arg("--force");
    }

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get the diverged commit from a particular git SHA or tag.
pub fn get_diverged_commit(refer: String, cwd: Option<String>) -> Option<String> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("merge-base").arg(&refer).arg("HEAD");
    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if !output.status.success() {
        return None;
    }

    let output = String::from_utf8(output.stdout).unwrap();

    Some(strip_trailing_newline(&output))
}

/// Get the current commit id
pub fn git_current_sha(cwd: Option<String>) -> String {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let hash = String::from_utf8(output.stdout).unwrap();
    strip_trailing_newline(&hash)
}

/// Get the previous commit id
pub fn git_previous_sha(cwd: Option<String>) -> String {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD~1");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let hash = String::from_utf8(output.stdout).unwrap();
    strip_trailing_newline(&hash)
}

/// Verify if as uncommited changes in the current working directory
pub fn git_workdir_unclean(cwd: Option<String>) -> bool {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("status").arg("--porcelain");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let output = String::from_utf8(output.stdout).unwrap();
    let result = strip_trailing_newline(&output);

    if result.is_empty() {
        return false;
    }

    true
}

/// Get the current branch name
pub fn git_current_branch(cwd: Option<String>) -> Option<String> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--abbrev-ref").arg("HEAD");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let output = String::from_utf8(output.stdout).unwrap();
    let result = strip_trailing_newline(&output);

    if result.is_empty() {
        return None;
    }

    Some(result)
}

/// Get the branch (last) name for a commit
pub fn git_branch_from_commit(commit: String, cwd: Option<String>) -> Option<String> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    // git --no-pager branch --no-color --no-column --format "%(refname:lstrip=2)" --contains <commit>
    let mut command = Command::new("git");
    command
        .arg("--no-pager")
        .arg("branch")
        .arg("--no-color")
        .arg("--no-column")
        .arg("--format")
        .arg(r#""%(refname:lstrip=2)""#)
        .arg("--contains")
        .arg(&commit);

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let output = String::from_utf8(output.stdout).unwrap();
    let result = strip_trailing_newline(&output);

    if result.is_empty() {
        return None;
    }

    Some(result)
}

/// Tags the current commit with a message
pub fn git_tag(
    tag: String,
    message: Option<String>,
    cwd: Option<String>,
) -> Result<bool, std::io::Error> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);
    let default_message = &tag;
    let msg = message.or(Some(default_message.to_string())).unwrap();

    let mut command = Command::new("git");
    command.arg("tag").arg("-a").arg(&tag).arg("-m").arg(&msg);

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Pushes all changes in the monorepo without verification and follow tags
pub fn git_push(cwd: Option<String>, follow_tags: Option<bool>) -> Result<bool, std::io::Error> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("push");

    if follow_tags.unwrap_or(false) {
        command.arg("--follow-tags");
    }

    command.arg("--no-verify");
    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

// Commit all changes in the monorepo
pub fn git_commit(
    mut message: String,
    body: Option<String>,
    footer: Option<String>,
    cwd: Option<String>,
) -> Result<bool, std::io::Error> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    if body.is_some() {
        message.push_str("\n\n");
        message.push_str(body.unwrap().as_str());
    }

    if footer.is_some() {
        message.push_str("\n\n");
        message.push_str(footer.unwrap().as_str());
    }

    let temp_dir = temp_dir();
    let temp_file_path = temp_dir.join("commit_message.txt");

    let mut file = File::create(&temp_file_path).unwrap();
    file.write_all(message.as_bytes()).unwrap();

    let file_path = temp_file_path.as_path();

    let mut command = Command::new("git");
    command
        .arg("commit")
        .arg("-F")
        .arg(&file_path.to_str().unwrap())
        .arg("--no-verify");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    remove_file(file_path).expect("Commit file not deleted");

    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Given a specific git sha, finds all files that have been modified
/// since the sha and returns the absolute filepaths.
pub fn git_all_files_changed_since_sha(sha: String, cwd: Option<String>) -> Vec<String> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");
    command
        .arg("--no-pager")
        .arg("diff")
        .arg("--name-only")
        .arg(format!("{}..", sha));
    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if !output.status.success() {
        return vec![];
    }

    let output = String::from_utf8(output.stdout).unwrap();
    let root = Path::new(&current_working_dir);

    output
        .split("\n")
        .filter(|item| !item.trim().is_empty())
        .map(|item| root.join(item))
        .filter(|item| item.exists())
        .map(|item| item.to_str().unwrap().to_string())
        .collect::<Vec<String>>()
}

/// Returns commits since a particular git SHA or tag.
/// If the "since" parameter isn't provided, all commits
/// from the dawn of man are returned
pub fn get_commits_since(
    cwd: Option<String>,
    since: Option<String>,
    relative: Option<String>,
) -> Vec<Commit> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    const DELIMITER: &str = r#"#=#"#;
    const BREAK_LINE: &str = r#"#+#"#;

    let mut command = Command::new("git");
    command
        .arg("--no-pager")
        .arg("log")
        .arg(format!(
            "--format={}%H{}%an{}%ae{}%ad{}%B{}",
            DELIMITER, DELIMITER, DELIMITER, DELIMITER, DELIMITER, BREAK_LINE
        ))
        .arg("--date=rfc2822");

    if let Some(since) = since {
        command.arg(format!("{}..", since));
    }

    if let Some(relative) = relative {
        command.arg("--");
        command.arg(&relative);
    }

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if !output.status.success() {
        return vec![];
    }

    let output = String::from_utf8(output.stdout).unwrap();

    output
        .split(BREAK_LINE)
        .filter(|item| !item.trim().is_empty())
        .map(|item| {
            let item_trimmed = item.trim();
            let items = item_trimmed.split(DELIMITER).collect::<Vec<&str>>();

            Commit {
                hash: items.get(1).unwrap().to_string(),
                author_name: items.get(2).unwrap().to_string(),
                author_email: items.get(3).unwrap().to_string(),
                author_date: items.get(4).unwrap().to_string(),
                message: items.get(5).unwrap().to_string(),
            }
        })
        .collect::<Vec<Commit>>()
}

/// Grabs the full list of all tags available on upstream or local
pub fn get_remote_or_local_tags(cwd: Option<String>, local: Option<bool>) -> Vec<RemoteTags> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut command = Command::new("git");

    match local {
        Some(true) => command.arg("show-ref").arg("--tags"),
        Some(false) => command.arg("ls-remote").arg("--tags").arg("origin"),
        None => command.arg("ls-remote").arg("--tags").arg("origin"),
    };

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    if !output.status.success() {
        return vec![];
    }

    let output = String::from_utf8(output.stdout).unwrap();

    #[cfg(windows)]
    const LINE_ENDING: &'static str = "\r\n";
    #[cfg(not(windows))]
    const LINE_ENDING: &'static str = "\n";

    output
        .trim()
        .split(LINE_ENDING)
        .filter(|tags| !tags.trim().is_empty())
        .map(|tags| {
            let hash_tags = Regex::new(r"\s+")
                .unwrap()
                .split(tags)
                .collect::<Vec<&str>>();

            RemoteTags {
                hash: hash_tags.get(0).unwrap().to_string(),
                tag: hash_tags.get(1).unwrap().to_string(),
            }
        })
        .collect::<Vec<RemoteTags>>()
}

/// Given an input of the "main" branch name,
/// returns all the files that have changed since the current branch was created
pub fn get_all_files_changed_since_branch(
    package_info: &Vec<PackageInfo>,
    branch: &String,
    cwd: Option<String>,
) -> Vec<String> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.unwrap_or(working_dir);

    let mut all_files = vec![];

    package_info.iter().for_each(|item| {
        let files = git_all_files_changed_since_sha(
            branch.to_string(),
            Some(current_working_dir.to_string()),
        );

        let pkg_files = files
            .iter()
            .filter(|file| file.starts_with(item.package_path.as_str()))
            .collect::<Vec<&String>>();

        all_files.append(
            &mut pkg_files
                .iter()
                .map(|file| file.to_string())
                .collect::<Vec<String>>(),
        );
    });

    all_files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_fetch_all() {
        let result = git_fetch_all(None, None);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_get_diverged_commit() {
        let result = get_diverged_commit(String::from("main"), None);
        assert_eq!(result.is_some(), true);
    }

    #[test]
    fn test_git_current_sha() {
        let result = git_current_sha(None);
        assert_eq!(result.is_empty(), false);
    }

    #[test]
    fn test_git_previous_sha() {
        let result = git_previous_sha(None);
        assert_eq!(result.is_empty(), false);
    }

    #[test]
    fn test_git_workdir_unclean() {
        let result = git_workdir_unclean(None);
        assert_eq!(result, false);
    }

    #[test]
    fn test_git_branch_from_commit() {
        let commit = git_current_sha(None);
        let result = git_branch_from_commit(commit, None);
        assert_eq!(result.is_some(), true);
    }

    #[test]
    fn test_get_commits_since() {
        let result = get_commits_since(
            None,
            Some(String::from("main")),
            Some(String::from("packages/package-a")),
        );
        let count = result.len();
        assert_eq!(count, count);
    }

    #[test]
    fn test_get_local_tags() {
        let result = get_remote_or_local_tags(None, Some(true));
        let count = result.len();
        assert_eq!(count, count);
    }

    #[test]
    fn test_get_remote_tags() {
        let result = get_remote_or_local_tags(None, Some(false));
        let count = result.len();
        assert_eq!(count, count);
    }

    #[test]
    fn test_git_all_files_changed_since_sha() {
        let result = git_all_files_changed_since_sha(String::from("main"), None);
        let count = result.len();
        assert_eq!(count, count);
    }
}
