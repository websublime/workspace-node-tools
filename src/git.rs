//! # Git
//!
//! This module provides a set of functions to interact with git.
#![allow(clippy::all)]
use execute::Execute;
use icu::collator::{Collator, CollatorOptions, Numeric, Strength};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::{
    env::temp_dir,
    fs::{remove_file, File},
    path::Path,
    process::{Command, Stdio},
};
use version_compare::{Cmp, Version};

use super::packages::PackageInfo;
use super::paths::get_project_root_path;
use super::utils::{package_scope_name_version, strip_trailing_newline};

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
/// A struct that represents a commit information
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
/// A struct that represents a remote tag information
pub struct RemoteTags {
    pub hash: String,
    pub tag: String,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublishTagInfo {
    pub hash: String,
    pub tag: String,
    pub package: String,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
/// A struct that represents a publish tag information
pub struct PublishTagInfo {
    pub hash: String,
    pub tag: String,
    pub package: String,
}

/// Stage all uncommitted changes
pub fn git_add_all(cwd: &String) -> Result<bool, std::io::Error> {
    let mut git_add = Command::new("git");

    git_add.current_dir(cwd.to_string()).arg("add").arg(".");

    git_add.stdout(Stdio::piped());
    git_add.stderr(Stdio::piped());

    let output = git_add.execute_output().unwrap();

    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Add a file to the git stage
pub fn git_add(cwd: &String, file: &String) -> Result<bool, std::io::Error> {
    let mut git_add = Command::new("git");

    git_add.current_dir(cwd.to_string()).arg("add").arg(file);

    git_add.stdout(Stdio::piped());
    git_add.stderr(Stdio::piped());

    let output = git_add.execute_output().unwrap();

    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Configure git user name and email
pub fn git_config(username: &String, email: &String, cwd: &String) -> Result<bool, std::io::Error> {
    let mut git_config_user = Command::new("git");

    git_config_user
        .current_dir(cwd.to_string())
        .arg("config")
        .arg("user.name")
        .arg(username);

    git_config_user.stdout(Stdio::piped());
    git_config_user.stderr(Stdio::piped());

    let output_user = git_config_user.execute_output().unwrap();

    let mut git_config_email = Command::new("git");
    git_config_email
        .current_dir(cwd.to_string())
        .arg("config")
        .arg("user.email")
        .arg(email);

    git_config_email.stdout(Stdio::piped());
    git_config_email.stderr(Stdio::piped());

    let output_email = git_config_email.execute_output().unwrap();
    let status = output_user.status.success() == output_email.status.success();

    if status {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Fetch everything from origin including tags
pub fn git_fetch_all(
    cwd: Option<String>,
    fetch_tags: Option<bool>,
) -> Result<bool, std::io::Error> {
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let mut command = Command::new("git");
    command.arg("rev-parse").arg("--short").arg("HEAD~1");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let hash = String::from_utf8(output.stdout).unwrap();

    strip_trailing_newline(&hash)
}

/// Get the first commit in a branch
pub fn git_first_sha(cwd: Option<String>, branch: Option<String>) -> String {
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let branch = match branch {
        Some(branch) => branch,
        None => String::from("main"),
    };

    let mut command = Command::new("git");
    command
        .arg("log")
        .arg(format!("{}..HEAD", branch))
        .arg("--online")
        .arg("--pretty=format:%h")
        .arg("|")
        .arg("tail")
        .arg("-1");

    command.current_dir(&current_working_dir);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let hash = String::from_utf8(output.stdout).unwrap();

    strip_trailing_newline(&hash)
}

/// Verify if as uncommited changes in the current working directory
pub fn git_workdir_unclean(cwd: Option<String>) -> bool {
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let mut command = Command::new("git");
    command
        .arg("--no-pager")
        .arg("diff")
        .arg("--name-only")
        .arg(format!("{}", sha));
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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

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

/// Grabs the last known publish tag info for a package
pub fn get_last_known_publish_tag_info_for_package(
    package_info: &PackageInfo,
    cwd: Option<String>,
) -> Option<PublishTagInfo> {
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    let mut remote_tags =
        get_remote_or_local_tags(Some(current_working_dir.to_string()), Some(false));
    let mut local_tags =
        get_remote_or_local_tags(Some(current_working_dir.to_string()), Some(true));

    /*let mut remote_tags = vec![
        RemoteTags {
            hash: String::from("ddd1fa69be3e6c6a8b2f18af8f8f5607106188db"),
            tag: String::from("refs/tags/@b2x/workspace-node@1.0.4")
        },
        RemoteTags {
            hash: String::from("c5353e1f3c9385c35f64e838a0a09dc4bb8f7b07"),
            tag: String::from("refs/tags/@b2x/workspace-node@1.0.2")
        }
    ];

    let mut local_tags = vec![
        RemoteTags {
            hash: String::from("4a16b15bb5cfeca493c79231452e94e56487d6b4"),
            tag: String::from("refs/tags/@b2x/workspace-node@0.9.9")
        },
        RemoteTags {
            hash: String::from("ee5f8209e6d3b06fbf5712e424652e909a4cb5c2"),
            tag: String::from("refs/tags/@b2x/workspace-node@1.0.5")
        }
    ];*/

    remote_tags.append(&mut local_tags);

    let mut options = CollatorOptions::new();
    options.strength = Some(Strength::Secondary);
    options.numeric = Some(Numeric::On);

    let collator = Collator::try_new(&Default::default(), options).unwrap();

    remote_tags.sort_by(|a, b| {
        let tag_a = a.tag.replace("refs/tags/", "");
        let tag_b = b.tag.replace("refs/tags/", "");

        collator.compare(&tag_b, &tag_a)
    });

    let package_tag = format!("{}@{}", package_info.name, package_info.version);

    let mut match_tag = remote_tags.iter().find(|item| {
        let tag = item.tag.replace("refs/tags/", "");
        let matches: Vec<&str> = tag.matches(&package_tag).collect();

        if matches.len() > 0 {
            return true;
        } else {
            return false;
        }
    });

    if match_tag.is_none() {
        let mut highest_tag = None;

        remote_tags.iter().for_each(|item| {
            let tag = &item.tag.replace("refs/tags/", "");

            if tag.contains(&package_info.name) {
                if highest_tag.is_none() {
                    highest_tag = Some(String::from(tag));
                }

                let high_tag = highest_tag.as_ref().unwrap();
                let current_tag_meta = package_scope_name_version(tag).unwrap();
                let highest_tag_meta = package_scope_name_version(high_tag).unwrap();

                let current_version = Version::from(&current_tag_meta.version).unwrap();
                let highest_version = Version::from(&highest_tag_meta.version).unwrap();

                if current_version.compare_to(&highest_version, Cmp::Gt) {
                    highest_tag = Some(String::from(tag));
                }
            }
        });

        if highest_tag.is_some() {
            let highest_tag = highest_tag.unwrap();
            let highest_tag_meta = package_scope_name_version(&highest_tag).unwrap();

            match_tag = remote_tags.iter().find(|item| {
                let tag = item.tag.replace("refs/tags/", "");
                let matches: Vec<&str> = tag.matches(&highest_tag_meta.full).collect();

                if matches.len() > 0 {
                    return true;
                } else {
                    return false;
                }
            });
        }
    }

    if match_tag.is_some() {
        let hash = &match_tag.unwrap().hash;
        let tag = &match_tag.unwrap().tag;
        let package = &package_info.name;

        return Some(PublishTagInfo {
            hash: hash.to_string(),
            tag: tag.to_string(),
            package: package.to_string(),
        });
    }

    None
}

/// Grabs the last known publish tag info for all packages in the monorepo
pub fn get_last_known_publish_tag_info_for_all_packages(
    package_info: &Vec<PackageInfo>,
    cwd: Option<String>,
) -> Vec<Option<PublishTagInfo>> {
    let root = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    git_fetch_all(Some(root.to_string()), Some(true)).expect("Fetch all tags");

    package_info
        .iter()
        .map(|item| get_last_known_publish_tag_info_for_package(&item, Some(root.to_string())))
        .filter(|item| item.is_some())
        .collect::<Vec<Option<PublishTagInfo>>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        manager::PackageManager, paths::get_project_root_path, utils::create_test_monorepo,
    };
    use std::fs::{remove_dir_all, File};

    #[test]
    fn test_git_fetch_all() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = git_fetch_all(project_root, None)?;
        assert_eq!(result, false);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_diverged_commit() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = get_diverged_commit(String::from("@scope/package-a@1.0.0"), project_root);

        assert!(result.is_some());
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_git_current_sha() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = git_current_sha(project_root);
        assert_eq!(result.is_empty(), false);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_git_previous_sha() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = git_previous_sha(project_root);
        assert_eq!(result.is_empty(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_git_workdir_unclean() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));
        let js_path = monorepo_dir.join("packages/package-a/index.js");

        let mut js_file = File::create(&js_path)?;
        js_file.write_all(r#"export const message = "hello";"#.as_bytes())?;

        let result = git_workdir_unclean(project_root);
        assert_eq!(result, true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_git_branch_from_commit() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let commit = git_current_sha(Some(project_root.as_ref().unwrap().to_string()));
        let result = git_branch_from_commit(commit, project_root);
        assert_eq!(result.is_some(), true);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_commits_since() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = get_commits_since(
            project_root,
            Some(String::from("main")),
            Some(String::from("packages/package-a")),
        );
        let count = result.len();

        assert_eq!(count, 0);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_local_tags() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = get_remote_or_local_tags(project_root, Some(true));
        let count = result.len();

        assert_eq!(count, 2);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_git_all_files_changed_since_sha() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let result = git_all_files_changed_since_sha(String::from("main"), project_root);
        let count = result.len();

        assert_eq!(count, 0);
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
