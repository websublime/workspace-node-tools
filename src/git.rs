use super::paths::get_project_root_path;
use super::utils::strip_trailing_newline;

use execute::Execute;
use std::process::{Command, Stdio};

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

    command.current_dir(current_working_dir);

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
    command.arg("merge-base").arg(refer).arg("HEAD");
    command.current_dir(current_working_dir);

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
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD");

    command.current_dir(current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let hash = String::from_utf8(output.stdout).unwrap();
    strip_trailing_newline(&hash)
}

/// Get the previous commit id
pub fn git_previous_sha(cwd: Option<String>) -> String {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD~1");

    command.current_dir(current_working_dir);

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

    command.current_dir(current_working_dir);

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

/// Get the branch (last) name for a commit
pub fn git_branch_from_commit(commit: String, cwd: Option<String>) -> Option<String> {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

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
        .arg(commit);

    command.current_dir(current_working_dir);

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
        let result = get_diverged_commit("main".to_string(), None);
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

}
