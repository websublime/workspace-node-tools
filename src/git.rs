use super::paths::get_project_root_path;
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

    Some(output)
}

/// Get the current commit id
pub fn git_current_sha(cwd: Option<String>) -> String {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD");

    command.current_dir(current_working_dir.clone());

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Get the previous commit id
pub fn git_previous_sha(cwd: Option<String>) -> String {
    let working_dir = get_project_root_path().unwrap();
    let current_working_dir = cwd.clone().unwrap_or(working_dir);

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD~1");

    command.current_dir(current_working_dir.clone());

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_fetch_all() {
        let result = git_fetch_all(None, None);
        assert_eq!(result.unwrap(), true);
    }
}
