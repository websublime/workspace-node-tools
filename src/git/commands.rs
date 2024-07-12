#![allow(clippy::all)]
use execute::Execute;
use icu::collator::{Collator, CollatorOptions, Numeric, Strength};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{
    env::temp_dir,
    fs::{remove_file, File},
    path::Path,
    process::{Command, Stdio},
};
use version_compare::{Cmp, Version};

use crate::{
    filesystem::paths::get_project_root_path,
    monorepo::{
        packages::PackageInfo, packages::PackageRepositoryInfo, utils::package_scope_name_version,
    },
};

use super::conventional::{ConventionalPackage, ConventionalPackageOptions};

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
pub struct PublishTagInfo {
    pub hash: String,
    pub tag: String,
    pub package: String,
}

pub struct Git;

impl Git {
    /**
     * Fetches all tracking information from origin.
     */
    pub fn fetch_all(cwd: Option<String>) -> Result<bool, std::io::Error> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut command = Command::new("git");
        command.arg("fetch").arg("origin");
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

    /**
     * Pulls in all tags from origin and forces local to be updated
     * @param {string} [cwd=monorepo root]
     */
    pub fn fetch_all_tags(cwd: Option<String>) -> Result<bool, std::io::Error> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut command = Command::new("git");
        command
            .arg("fetch")
            .arg("origin")
            .arg("--tags")
            .arg("--force");
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

    /**
     * Returns commits since a particular git SHA or tag.
     * If the "since" parameter isn't provided, all commits
     * from the dawn of man are returned
     */
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
            command.arg(relative);
        }

        command.current_dir(current_working_dir);

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

    /**
     * Grabs the full list of all tags available on upstream or local
     */
    pub fn get_remote_or_local_tags(cwd: Option<String>, local: Option<bool>) -> Vec<RemoteTags> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut command = Command::new("git");

        match local {
            Some(true) => command.arg("show-ref").arg("--tags"),
            Some(false) => command.arg("ls-remote").arg("--tags").arg("origin"),
            None => command.arg("ls-remote").arg("--tags").arg("origin"),
        };

        command.current_dir(current_working_dir);

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

    pub fn get_last_known_publish_tag_info_for_package(
        package_info: PackageInfo,
        cwd: Option<String>,
    ) -> Option<PublishTagInfo> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut remote_tags =
            Self::get_remote_or_local_tags(Some(current_working_dir.clone()), Some(false));
        let mut local_tags = Self::get_remote_or_local_tags(Some(current_working_dir), Some(true));

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
                let tag = item.tag.replace("refs/tags/", "");

                if tag.contains(&package_info.name) {
                    if highest_tag.is_none() {
                        highest_tag = Some(tag.clone());
                    }

                    let current_tag_meta = package_scope_name_version(&tag).unwrap();
                    let highest_tag_meta =
                        package_scope_name_version(&highest_tag.clone().unwrap()).unwrap();

                    let current_version = Version::from(&current_tag_meta.version).unwrap();
                    let highest_version = Version::from(&highest_tag_meta.version).unwrap();

                    if current_version.compare_to(&highest_version, Cmp::Gt) {
                        highest_tag = Some(tag);
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
            return Some(PublishTagInfo {
                hash: match_tag.unwrap().hash.clone(),
                tag: match_tag.unwrap().tag.clone(),
                package: package_info.name,
            });
        }

        None
    }

    /**
     * Grabs the last known publish tag info for all packages in the monorepo
     */
    pub fn get_last_known_publish_tag_info_for_all_packages(
        package_info: Vec<PackageInfo>,
        cwd: Option<String>,
    ) -> Vec<Option<PublishTagInfo>> {
        Self::fetch_all_tags(cwd.clone()).expect("Fetch all tags");

        package_info
            .iter()
            .map(|item| {
                Self::get_last_known_publish_tag_info_for_package(item.clone(), cwd.clone())
            })
            .filter(|item| item.is_some())
            .collect::<Vec<Option<PublishTagInfo>>>()
    }

    /**
     * Given a specific git sha, finds all files that have been modified
     * since the sha and returns the absolute filepaths.
     */
    pub fn git_all_files_changed_since_sha(sha: String, cwd: Option<String>) -> Vec<String> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut command = Command::new("git");
        command
            .arg("--no-pager")
            .arg("diff")
            .arg("--name-only")
            .arg(format!("{}..", sha));
        command.current_dir(current_working_dir.clone());

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

    /**
     * Given an input of parsed git tag infos,
     * returns all the files that have changed since any of these git tags
     * have occured, with duplicates removed.
     */
    pub fn get_all_files_changed_since_tag_infos(
        package_info: Vec<PackageInfo>,
        tag_info: Vec<PublishTagInfo>,
        cwd: Option<String>,
    ) -> Vec<String> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut all_files = vec![];

        package_info.iter().for_each(|item| {
            let tag = tag_info.iter().find(|tag| tag.package == item.name);

            match tag {
                Some(tag) => {
                    let files = Self::git_all_files_changed_since_sha(
                        tag.hash.clone(),
                        Some(current_working_dir.clone()),
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
                }
                None => {}
            }
        });

        all_files
    }

    /**
     * Given an input of the "main" branch name,
     * returns all the files that have changed since the current branch was created
     */
    pub fn get_all_files_changed_since_branch(
        package_info: Vec<PackageInfo>,
        branch: String,
        cwd: Option<String>,
    ) -> Vec<String> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.unwrap_or(working_dir);

        let mut all_files = vec![];

        package_info.iter().for_each(|item| {
            let files = Self::git_all_files_changed_since_sha(
                branch.clone(),
                Some(current_working_dir.clone()),
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

    // git diff-tree --no-commit-id --name-only -r origin/main..HEAD
    // git --no-pager diff --name-only HEAD~1

    pub fn get_conventional_for_package(
        package_info: PackageInfo,
        no_fetch_all: Option<bool>,
        cwd: Option<String>,
        conventional_options: Option<ConventionalPackageOptions>,
    ) -> ConventionalPackage {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.clone().unwrap_or(working_dir);

        if no_fetch_all.is_none() {
            Self::fetch_all(cwd.clone()).expect("Fetch all");
        }

        let tag_info =
            Self::get_last_known_publish_tag_info_for_package(package_info.clone(), cwd.clone());

        let hash = match tag_info {
            Some(tag) => Some(tag.hash),
            None => None,
        };

        let convention_options = match conventional_options {
            Some(options) => ConventionalPackageOptions {
                version: options.version.or(Some(String::from("0.0.0"))),
                title: options.title,
            },
            None => ConventionalPackageOptions {
                version: Some(String::from("0.0.0")),
                title: None,
            },
        };

        let repo_info = package_info.repository_info.clone();
        let repository_info = match repo_info {
            Some(info) => info,
            None => PackageRepositoryInfo {
                orga: String::from("my-orga"),
                project: String::from("my-repo"),
                domain: String::from("https://github.com"),
            },
        };

        let commits_since = Self::get_commits_since(
            Some(current_working_dir),
            hash,
            Some(package_info.package_relative_path.clone()),
        );
        let mut conventional_package = ConventionalPackage::new(package_info);
        let conventional_config = conventional_package.define_config(
            repository_info.orga,
            repository_info.project,
            repository_info.domain,
            convention_options.title,
            None,
        );
        let config_git = conventional_config.git.clone();
        let conventional_package_live_one = conventional_package.clone();
        let conventional_package_live_two = conventional_package.clone();

        let conventional_commits =
            conventional_package_live_one.process_commits(&commits_since, &config_git);
        let changelog = conventional_package_live_two.generate_changelog(
            &conventional_commits,
            &conventional_config,
            convention_options.version,
        );

        conventional_package.changelog_output = changelog;
        conventional_package.conventional_commits =
            serde_json::to_value(&conventional_commits).unwrap();
        conventional_package.conventional_config = serde_json::to_value(&config_git).unwrap();

        conventional_package
    }

    pub fn git_commit(
        mut message: String,
        body: Option<String>,
        footer: Option<String>,
        cwd: Option<String>,
    ) -> Result<bool, std::io::Error> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.clone().unwrap_or(working_dir);

        if body.is_some() {
            message.push_str("\n\n");
            message.push_str(body.unwrap().as_str());
        }

        if footer.is_some() {
            message.push_str("\n\n");
            message.push_str(footer.unwrap().as_str());
        }

        let temp_file = temp_dir().join("commit_message.txt");
        let mut file = File::create(temp_file.clone()).unwrap();
        file.write_all(message.as_bytes()).unwrap();

        let mut command = Command::new("git");
        command
            .arg("commit")
            .arg("-F")
            .arg(temp_file.to_str().unwrap())
            .arg("--no-verify");

        command.current_dir(current_working_dir.clone());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output().unwrap();

        remove_file(temp_file).expect("Commit file not deleted");

        if output.status.success() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn git_push(cwd: Option<String>) -> Result<bool, std::io::Error> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.clone().unwrap_or(working_dir);

        let mut command = Command::new("git");
        command.arg("push").arg("--no-verify");

        command.current_dir(current_working_dir.clone());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output().unwrap();

        if output.status.success() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn git_tag(
        tag: String,
        message: Option<String>,
        cwd: Option<String>,
    ) -> Result<bool, std::io::Error> {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.clone().unwrap_or(working_dir);
        let msg = message.or(Some(tag.clone())).unwrap();

        let mut command = Command::new("git");
        command
            .arg("tag")
            .arg("-a")
            .arg(tag.clone())
            .arg("-m")
            .arg(msg);

        command.current_dir(current_working_dir.clone());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output().unwrap();

        if output.status.success() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

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

    pub fn git_workdir_unclean(cwd: Option<String>) -> bool {
        let working_dir = get_project_root_path().unwrap();
        let current_working_dir = cwd.clone().unwrap_or(working_dir);

        let mut command = Command::new("git");
        command.arg("status").arg("--porcelain");

        command.current_dir(current_working_dir.clone());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output().unwrap();

        let output = String::from_utf8(output.stdout).unwrap();

        if output.trim().is_empty() {
            return false;
        }

        true
    }

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

        command.current_dir(current_working_dir.clone());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output().unwrap();

        let output = String::from_utf8(output.stdout).unwrap();

        if output.trim().is_empty() {
            return None;
        }

        Some(output.trim().split("\n").last().unwrap_or_default().to_string())
    }
}
