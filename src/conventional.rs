//! # Conventional
//!
//! This module is responsible for generating changelog output for a package based on conventional commits.
#![allow(clippy::all)]
use git_cliff_core::{
    changelog::Changelog,
    commit::{Commit as GitCommit, Signature},
    config::{
        Bump, ChangelogConfig, CommitParser, Config, GitConfig, Remote, RemoteConfig, TextProcessor,
    },
    release::Release,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::git::{
    get_commits_since, get_last_known_publish_tag_info_for_package, git_fetch_all, Commit,
};
use super::packages::PackageInfo;
use super::packages::PackageRepositoryInfo;
use super::paths::get_project_root_path;

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConventionalPackage {
    pub package_info: PackageInfo,
    pub conventional_config: Value,
    pub conventional_commits: Value,
    pub changelog_output: String,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConventionalPackage {
    pub package_info: PackageInfo,
    pub conventional_config: Value,
    pub conventional_commits: Value,
    pub changelog_output: String,
}

#[cfg(feature = "napi")]
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ConventionalPackageOptions {
    pub version: Option<String>,
    pub title: Option<String>,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone)]
pub struct ConventionalPackageOptions {
    pub version: Option<String>,
    pub title: Option<String>,
}

fn process_commits<'a>(commits: &Vec<Commit>, config: &GitConfig) -> Vec<GitCommit<'a>> {
    commits
        .iter()
        .map(|commit| {
            let timestamp = chrono::DateTime::parse_from_rfc2822(&commit.author_date).unwrap();

            let git_commit = GitCommit {
                id: commit.hash.to_string(),
                message: commit.message.to_string(),
                author: Signature {
                    name: Some(commit.author_name.to_string()),
                    email: Some(commit.author_email.to_string()),
                    timestamp: timestamp.timestamp(),
                },
                ..GitCommit::default()
            };

            git_commit.process(config).unwrap()
        })
        .collect::<Vec<GitCommit>>()
}

fn define_config(
    owner: String,
    repo: String,
    domain: String,
    title: Option<String>,
    options: &Option<Config>,
) -> Config {
    let github_url = format!("{}/{}/{}", domain, owner, repo);

    let cliff_config = match options {
        Some(config) => config.to_owned(),
        None => {
            let config = Config {
                bump: Bump::default(),
                remote: RemoteConfig {
                    github: Remote {
                        owner: String::from(owner),
                        repo: String::from(repo),
                        token: None,
                    },
                    ..RemoteConfig::default()
                },
                changelog: ChangelogConfig {
                    header: title,
                    body: Some(String::from(
                        r#"
                        {%- macro remote_url() -%}
                          <REPO>
                        {%- endmacro -%}

                        {% macro print_commit(commit) -%}
                            - {% if commit.scope %}*({{ commit.scope }})* {% endif %}{% if commit.breaking %}[**breaking**] {% endif %}{{ commit.message | upper_first }} - ([{{ commit.id | truncate(length=7, end="") }}]({{ self::remote_url() }}/commit/{{ commit.id }}))
                        {% endmacro -%}

                        {% if version %}
                            {% if previous.version %}
                                ## [{{ version | trim_start_matches(pat="v") }}]
                                  ({{ self::remote_url() }}/compare/{{ previous.version }}..{{ version }}) - {{ now() | date(format="%Y-%m-%d") }}
                            {% else %}
                                ## [{{ version | trim_start_matches(pat="v") }}] - {{ now() | date(format="%Y-%m-%d") }}
                            {% endif %}
                        {% else %}
                            ## [unreleased]
                        {% endif %}

                        {% for group, commits in commits | group_by(attribute="group") %}
                            ### {{ group | striptags | trim | upper_first }}
                            {% for commit in commits
                            | filter(attribute="scope")
                            | sort(attribute="scope") %}
                                {{ self::print_commit(commit=commit) }}
                            {%- endfor -%}
                            {% raw %}
                            {% endraw %}
                            {%- for commit in commits %}
                                {%- if not commit.scope -%}
                                    {{ self::print_commit(commit=commit) }}
                                {% endif -%}
                            {% endfor -%}
                        {% endfor %}"#,
                    )),
                    footer: Some(String::from(
                        r#"-- Total Releases: {{ releases | length }} --"#,
                    )),
                    trim: Some(true),
                    postprocessors: Some(vec![TextProcessor {
                        pattern: Regex::new("<REPO>").expect("failed to compile regex"),
                        replace: Some(String::from(github_url)),
                        replace_command: None,
                    }]),
                },
                git: GitConfig {
                    commit_parsers: Some(vec![
                        CommitParser {
                            message: Some(Regex::new("^feat").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 0 -->⛰️  Features")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(Regex::new("^fix").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 1 -->🐛  Bug Fixes")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(Regex::new("^doc").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 3 -->📚 Documentation")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(Regex::new("^perf").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 4 -->⚡ Performance")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(
                                Regex::new("^refactor\\(clippy\\)")
                                    .expect("failed to compile regex"),
                            ),
                            skip: Some(true),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(
                                Regex::new("^refactor").expect("failed to compile regex"),
                            ),
                            group: Some(String::from("<!-- 2 -->🚜 Refactor")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(Regex::new("^style").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 5 -->🎨 Styling")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(Regex::new("^test").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 6 -->🧪 Testing")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(
                                Regex::new("^chore|^ci").expect("failed to compile regex"),
                            ),
                            group: Some(String::from("<!-- 7 -->⚙️ Miscellaneous Tasks")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            body: Some(Regex::new(".*security").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 8 -->🛡️ Security")),
                            ..CommitParser::default()
                        },
                        CommitParser {
                            message: Some(Regex::new("^revert").expect("failed to compile regex")),
                            group: Some(String::from("<!-- 9 -->◀️ Revert")),
                            ..CommitParser::default()
                        },
                    ]),
                    protect_breaking_commits: Some(false),
                    filter_commits: Some(false),
                    tag_pattern: Some(
                        Regex::new("^((?:@[^/@]+/)?[^/@]+)(?:@([^/]+))?$")
                            .expect("failed to compile regex"),
                    ),
                    skip_tags: Some(
                        Regex::new("beta|alpha|snapshot").expect("failed to compile regex"),
                    ),
                    ignore_tags: Some(
                        Regex::new("rc|beta|alpha|snapshot").expect("failed to compile regex"),
                    ),
                    topo_order: Some(false),
                    sort_commits: Some(String::from("newest")),
                    ..GitConfig::default()
                },
            };

            config
        }
    };

    cliff_config
}

fn generate_changelog(
    commits: &Vec<GitCommit>,
    config: &Config,
    version: Option<String>,
) -> String {
    let releases = Release {
        version,
        commits: commits.to_vec().to_owned(),
        ..Release::default()
    };

    let changelog = Changelog::new(vec![releases], config);
    let mut changelog_output = Vec::new();

    changelog.unwrap().generate(&mut changelog_output).unwrap();

    String::from_utf8(changelog_output).unwrap_or_default()
}

/// Give info about commits in a package, generate changelog output
pub fn get_conventional_for_package(
    package_info: &PackageInfo,
    no_fetch_all: Option<bool>,
    cwd: Option<String>,
    conventional_options: &Option<ConventionalPackageOptions>,
) -> ConventionalPackage {
    let current_working_dir = match cwd {
        Some(dir) => get_project_root_path(Some(PathBuf::from(dir))).unwrap(),
        None => get_project_root_path(None).unwrap(),
    };

    if no_fetch_all.is_some() {
        git_fetch_all(Some(current_working_dir.to_string()), no_fetch_all).expect("Fetch all");
    }

    let tag_info = get_last_known_publish_tag_info_for_package(
        package_info,
        Some(current_working_dir.to_string()),
    );

    let hash = match tag_info {
        Some(tag) => Some(tag.hash),
        None => None,
    };

    let conventional_default_options = match conventional_options {
        Some(options) => {
            let opt_version = options.version.as_ref();
            let default_version = &String::from("0.0.0");
            let version = opt_version.unwrap_or(default_version);

            let opt_title = options.title.as_ref();
            let default_title = &String::from("");
            let title = opt_title.unwrap_or(default_title);

            ConventionalPackageOptions {
                version: Some(version.to_string()),
                title: Some(title.to_string()),
            }
        }
        None => ConventionalPackageOptions {
            version: Some(String::from("0.0.0")),
            title: None,
        },
    };

    let repo_info = &package_info.repository_info;
    let repository_info = match repo_info {
        Some(info) => info.to_owned(),
        None => PackageRepositoryInfo {
            orga: String::from("my-orga"),
            project: String::from("my-repo"),
            domain: String::from("https://github.com"),
        },
    };

    let package_relative_path = &package_info.package_relative_path;
    let commits_since = get_commits_since(
        Some(current_working_dir.to_string()),
        hash,
        Some(package_relative_path.to_string()),
    );

    let pkg_info = package_info;
    let mut conventional_package = ConventionalPackage {
        package_info: pkg_info.to_owned(),
        conventional_config: json!({}),
        conventional_commits: json!([]),
        changelog_output: String::new(),
    };

    let orga = &repository_info.orga;
    let project = &repository_info.project;
    let domain = &repository_info.domain;

    let conventional_config = define_config(
        orga.to_string(),
        project.to_string(),
        domain.to_string(),
        conventional_default_options.title,
        &None,
    );

    let conventional_commits = process_commits(&commits_since, &conventional_config.git);

    let changelog = generate_changelog(
        &conventional_commits,
        &conventional_config,
        conventional_default_options.version,
    );

    let changelog_output = &changelog.to_string();
    conventional_package.changelog_output = changelog_output.to_string();
    conventional_package.conventional_commits =
        serde_json::to_value(&conventional_commits).unwrap();
    conventional_package.conventional_config =
        serde_json::to_value(&conventional_config.git).unwrap();

    conventional_package
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::manager::PackageManager;
    use crate::packages::get_packages;
    use crate::paths::get_project_root_path;
    use crate::utils::create_test_monorepo;
    use std::fs::remove_dir_all;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    fn create_package_change(monorepo_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let js_path = monorepo_dir.join("packages/package-b/index.js");

        let branch = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("-b")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git branch problem");

        branch.wait_with_output()?;

        let mut js_file = File::create(&js_path)?;
        js_file
            .write_all(r#"export const message = "hello";"#.as_bytes())
            .unwrap();

        let add = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("add")
            .arg(".")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git add problem");

        add.wait_with_output()?;

        let commit = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("commit")
            .arg("-m")
            .arg("feat: message to the world")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git commit problem");

        commit.wait_with_output()?;

        let main = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("checkout")
            .arg("main")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git checkout problem");

        main.wait_with_output()?;

        let merge = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("merge")
            .arg("feat/message")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git merge problem");

        merge.wait_with_output()?;

        let tag_b = Command::new("git")
            .current_dir(&monorepo_dir)
            .arg("tag")
            .arg("-a")
            .arg("@scope/package-b@1.1.0")
            .arg("-m")
            .arg("chore: release package-b@1.1.0")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Git tag problem");

        tag_b.wait_with_output()?;

        Ok(())
    }

    #[test]
    fn test_get_conventional_for_package() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let ref root = project_root.unwrap().to_string();

        let packages = get_packages(Some(root.to_string()));
        let package = packages.first();

        let conventional =
            get_conventional_for_package(package.unwrap(), None, Some(root.to_string()), &None);

        assert_eq!(conventional.package_info, package.unwrap().to_owned());
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn test_get_conventional_for_package_with_changes() -> Result<(), Box<dyn std::error::Error>> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        create_package_change(monorepo_dir)?;

        let ref root = project_root.unwrap().to_string();

        let packages = get_packages(Some(root.to_string()));
        let package = packages.first();

        let conventional =
            get_conventional_for_package(package.unwrap(), None, Some(root.to_string()), &None);

        assert_eq!(
            conventional
                .changelog_output
                .contains("Message to the world"),
            true
        );
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }
}
