#![allow(clippy::all)]
use crate::{git::commands::Commit, monorepo::packages::PackageInfo};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use git_cliff_core::{
    changelog::Changelog,
    commit::{Commit as GitCommit, Signature},
    config::{
        Bump, ChangelogConfig, CommitParser, Config, GitConfig, Remote, RemoteConfig, TextProcessor,
    },
    release::Release,
};
use regex::Regex;

#[napi(object)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConventionalPackage {
    pub package_info: PackageInfo,
    pub conventional_config: Value,
    pub conventional_commits: Value,
    pub changelog_output: String,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ConventionalPackageOptions {
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub version: Option<String>,
    pub domain: Option<String>,
    pub title: Option<String>,
}

impl ConventionalPackage {
    pub fn new(package_info: PackageInfo) -> Self {
        ConventionalPackage {
            package_info,
            conventional_config: json!({}),
            conventional_commits: json!([]),
            changelog_output: String::new(),
        }
    }

    pub fn define_config(
        &self,
        owner: String,
        repo: String,
        domain: String,
        title: Option<String>,
        options: Option<Config>,
    ) -> Config {
        let github_url = format!("{}/{}/{}", domain, owner, repo);

        let cliff_config = match options {
            Some(config) => config,
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
                                message: Some(
                                    Regex::new("^feat").expect("failed to compile regex"),
                                ),
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
                                message: Some(
                                    Regex::new("^perf").expect("failed to compile regex"),
                                ),
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
                                message: Some(
                                    Regex::new("^style").expect("failed to compile regex"),
                                ),
                                group: Some(String::from("<!-- 5 -->🎨 Styling")),
                                ..CommitParser::default()
                            },
                            CommitParser {
                                message: Some(
                                    Regex::new("^test").expect("failed to compile regex"),
                                ),
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
                                body: Some(
                                    Regex::new(".*security").expect("failed to compile regex"),
                                ),
                                group: Some(String::from("<!-- 8 -->🛡️ Security")),
                                ..CommitParser::default()
                            },
                            CommitParser {
                                message: Some(
                                    Regex::new("^revert").expect("failed to compile regex"),
                                ),
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

    pub fn process_commits(&self, commits: &Vec<Commit>, config: &GitConfig) -> Vec<GitCommit> {
        commits
            .iter()
            .map(|commit| {
                let timestamp = chrono::DateTime::parse_from_rfc2822(&commit.author_date).unwrap();

                let git_commit = GitCommit {
                    id: commit.hash.clone(),
                    message: commit.message.clone(),
                    author: Signature {
                        name: Some(commit.author_name.clone()),
                        email: Some(commit.author_email.clone()),
                        timestamp: timestamp.timestamp(),
                    },
                    ..GitCommit::default()
                };

                git_commit.process(config).unwrap()
            })
            .collect::<Vec<GitCommit>>()
    }

    pub fn generate_changelog(
        &self,
        commits: &Vec<GitCommit>,
        config: &Config,
        version: Option<String>,
    ) -> String {
        let releases = Release {
            version,
            commits: commits.clone(),
            ..Release::default()
        };

        let changelog = Changelog::new(vec![releases], config);
        let mut changelog_output = Vec::new();

        changelog.unwrap().generate(&mut changelog_output).unwrap();

        String::from_utf8(changelog_output).unwrap_or_default()
    }
}
