#![allow(clippy::all)]
#![allow(dead_code)]

use regex::Regex;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::fs::{create_dir, File, OpenOptions};
#[cfg(test)]
use std::io::{BufWriter, Write};

#[cfg(test)]
use rand::distributions::Alphanumeric;
#[cfg(test)]
use rand::{thread_rng, Rng};

#[cfg(test)]
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;

#[cfg(test)]
use super::manager::PackageManager;
#[cfg(test)]
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Package scope metadata extracted from a package name.
pub struct PackageScopeMetadata {
    pub full: String,
    pub name: String,
    pub version: String,
    pub path: Option<String>,
}

/// Extracts the package scope name and version from a package name.
pub(crate) fn package_scope_name_version(pkg_name: &str) -> Option<PackageScopeMetadata> {
    let regex = Regex::new("^((?:@[^/@]+/)?[^/@]+)(?:@([^/]+))?(/.*)?$").unwrap();

    let matches = regex.captures(pkg_name).unwrap();

    if matches.len() > 0 {
        return Some(PackageScopeMetadata {
            full: matches.get(0).map_or("", |m| m.as_str()).to_string(),
            name: matches.get(1).map_or("", |m| m.as_str()).to_string(),
            version: matches.get(2).map_or("", |m| m.as_str()).to_string(),
            path: matches
                .get(3)
                .map_or(None, |m| Some(m.as_str().to_string())),
        });
    }

    None
}

/// Strips the trailing newline from a string.
pub(crate) fn strip_trailing_newline(input: &String) -> String {
    input
        .strip_suffix("\r\n")
        .or(input.strip_suffix("\n"))
        .unwrap_or(input)
        .trim()
        .to_string()
}

#[cfg(test)]
pub(crate) fn create_test_monorepo(
    package_manager: &PackageManager,
) -> Result<std::path::PathBuf, std::io::Error> {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    let temp_dir = std::env::temp_dir();
    let monorepo_temp_dir = temp_dir.join(format!("monorepo-{}", rand_string));
    let monorepo_package_json = monorepo_temp_dir.join("package.json");

    let monorepo_packages_dir = monorepo_temp_dir.join("packages");
    let monorepo_package_a_dir = monorepo_packages_dir.join("package-a");
    let monorepo_package_b_dir = monorepo_packages_dir.join("package-b");
    let monorepo_package_c_dir = monorepo_packages_dir.join("package-c");
    let monorepo_package_d_dir = monorepo_packages_dir.join("package-d");

    create_dir(&monorepo_temp_dir)?;
    create_dir(&monorepo_packages_dir)?;
    create_dir(&monorepo_package_a_dir)?;
    create_dir(&monorepo_package_b_dir)?;
    create_dir(&monorepo_package_c_dir)?;
    create_dir(&monorepo_package_d_dir)?;

    #[cfg(not(windows))]
    std::fs::set_permissions(&monorepo_temp_dir, std::fs::Permissions::from_mode(0o777))?;

    let monorepo_root_json = r#"
    {
        "name": "@scope/root",
        "version": "0.0.0",
        "workspaces": [
            "packages/package-a",
            "packages/package-b",
            "packages/package-c",
            "packages/package-d"
        ]
    }"#;
    let package_root_json = serde_json::from_str::<serde_json::Value>(monorepo_root_json).unwrap();
    let monorepo_package_root_json_file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(&monorepo_package_json.as_path())
        .unwrap();
    let monorepo_root_json_writer = BufWriter::new(monorepo_package_root_json_file);
    serde_json::to_writer_pretty(monorepo_root_json_writer, &package_root_json).unwrap();

    let package_a_json = r#"
    {
        "name": "@scope/package-a",
        "version": "1.0.0",
        "description": "My new package A",
        "main": "index.mjs",
        "module": "./dist/index.mjs",
        "exports": {
          ".": {
            "types": "./dist/index.d.ts",
            "default": "./dist/index.mjs"
          }
        },
        "typesVersions": {
          "*": {
            "index.d.ts": [
              "./dist/index.d.ts"
            ]
          }
        },
        "repository": {
          "url": "git+ssh://git@github.com/websublime/workspace-node-binding-tools.git",
          "type": "git"
        },
        "scripts": {
          "test": "echo \"Error: no test specified\" && exit 1",
          "dev": "node index.mjs"
        },
        "dependencies": {
          "@scope/package-b": "1.0.0"
        },
        "keywords": [],
        "author": "Author",
        "license": "ISC"
    }"#;
    let package_a_json = serde_json::from_str::<serde_json::Value>(package_a_json).unwrap();
    let monorepo_package_a_json_file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(&monorepo_package_a_dir.join("package.json").as_path())
        .unwrap();
    let monorepo_package_a_json_writer = BufWriter::new(monorepo_package_a_json_file);
    serde_json::to_writer_pretty(monorepo_package_a_json_writer, &package_a_json).unwrap();

    let package_b_json = r#"
    {
        "name": "@scope/package-b",
        "version": "1.0.0",
        "description": "My new package B",
        "main": "index.mjs",
        "module": "./dist/index.mjs",
        "exports": {
          ".": {
            "types": "./dist/index.d.ts",
            "default": "./dist/index.mjs"
          }
        },
        "typesVersions": {
          "*": {
            "index.d.ts": [
              "./dist/index.d.ts"
            ]
          }
        },
        "repository": {
          "url": "git+ssh://git@github.com/websublime/workspace-node-binding-tools.git",
          "type": "git"
        },
        "scripts": {
          "test": "echo \"Error: no test specified\" && exit 1",
          "dev": "node index.mjs"
        },
        "keywords": [],
        "author": "Author",
        "license": "ISC"
    }"#;
    let package_b_json = serde_json::from_str::<serde_json::Value>(package_b_json).unwrap();
    let monorepo_package_b_json_file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(&monorepo_package_b_dir.join("package.json").as_path())
        .unwrap();
    let monorepo_package_b_json_writer = BufWriter::new(monorepo_package_b_json_file);
    serde_json::to_writer_pretty(monorepo_package_b_json_writer, &package_b_json).unwrap();

    let package_c_json = r#"
    {
        "name": "@scope/package-c",
        "version": "1.0.0",
        "description": "My new package C",
        "main": "index.mjs",
        "module": "./dist/index.mjs",
        "exports": {
          ".": {
            "types": "./dist/index.d.ts",
            "default": "./dist/index.mjs"
          }
        },
        "typesVersions": {
          "*": {
            "index.d.ts": [
              "./dist/index.d.ts"
            ]
          }
        },
        "repository": {
          "url": "git+ssh://git@github.com/websublime/workspace-node-binding-tools.git",
          "type": "git"
        },
        "scripts": {
          "test": "echo \"Error: no test specified\" && exit 1",
          "dev": "node index.mjs"
        },
        "keywords": [],
        "author": "Author",
        "license": "ISC"
    }"#;
    let package_c_json = serde_json::from_str::<serde_json::Value>(package_c_json).unwrap();
    let monorepo_package_c_json_file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(&monorepo_package_c_dir.join("package.json").as_path())
        .unwrap();
    let monorepo_package_c_json_writer = BufWriter::new(monorepo_package_c_json_file);
    serde_json::to_writer_pretty(monorepo_package_c_json_writer, &package_c_json).unwrap();

    let package_d_json = r#"
    {
        "name": "@scope/package-d",
        "version": "1.0.0",
        "description": "My new package D",
        "main": "index.mjs",
        "module": "./dist/index.mjs",
        "exports": {
          ".": {
            "types": "./dist/index.d.ts",
            "default": "./dist/index.mjs"
          }
        },
        "typesVersions": {
          "*": {
            "index.d.ts": [
              "./dist/index.d.ts"
            ]
          }
        },
        "repository": {
          "url": "git+ssh://git@github.com/websublime/workspace-node-binding-tools.git",
          "type": "git"
        },
        "scripts": {
          "test": "echo \"Error: no test specified\" && exit 1",
          "dev": "node index.mjs"
        },
        "dependencies": {
          "@scope/package-a": "1.0.0"
        },
        "keywords": [],
        "author": "Author",
        "license": "ISC"
    }"#;
    let package_d_json = serde_json::from_str::<serde_json::Value>(package_d_json).unwrap();
    let monorepo_package_d_json_file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(&monorepo_package_d_dir.join("package.json").as_path())
        .unwrap();
    let monorepo_package_d_json_writer = BufWriter::new(monorepo_package_d_json_file);
    serde_json::to_writer_pretty(monorepo_package_d_json_writer, &package_d_json).unwrap();

    match package_manager {
        PackageManager::Yarn => {
            let yarn_lock = monorepo_temp_dir.join("yarn.lock");
            File::create(&yarn_lock)?;
        }
        PackageManager::Pnpm => {
            let pnpm_lock = monorepo_temp_dir.join("pnpm-lock.yaml");
            let pnpm_workspace = monorepo_temp_dir.join("pnpm-workspace.yaml");

            let mut lock_file = File::create(&pnpm_lock)?;
            lock_file.write_all(r#"lockfileVersion: '9.0'"#.as_bytes())?;

            let mut workspace_file = File::create(&pnpm_workspace)?;
            workspace_file.write_all(
                r#"
                packages:
                  - "packages/*"
            "#
                .as_bytes(),
            )?;
        }
        PackageManager::Bun => {
            let bun_lock = monorepo_temp_dir.join("bun.lockb");
            File::create(&bun_lock)?;
        }
        PackageManager::Npm => {
            let npm_lock = monorepo_temp_dir.join("package-lock.json");
            File::create(&npm_lock)?;
        }
    }

    let init = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("init")
        .arg("--initial-branch")
        .arg("main")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git init problem");

    init.wait_with_output()?;

    let config_email = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("config")
        .arg("user.email")
        .arg("machine@websublime.dev")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git config user email problem");

    config_email.wait_with_output()?;

    let config_name = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("config")
        .arg("user.name")
        .arg("Sublime Machine")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git config user name problem");

    config_name.wait_with_output()?;

    let add = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("add")
        .arg(".")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git add problem");

    add.wait_with_output()?;

    let commit = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("commit")
        .arg("-m")
        .arg("feat: project creation")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git commit problem");

    commit.wait_with_output()?;

    let tag_a = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("tag")
        .arg("-a")
        .arg("@scope/package-a@1.0.0")
        .arg("-m")
        .arg("chore: release package-a@1.0.0")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git tag problem");

    tag_a.wait_with_output()?;

    let tag_b = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("tag")
        .arg("-a")
        .arg("@scope/package-b@1.0.0")
        .arg("-m")
        .arg("chore: release package-b@1.0.0")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git tag problem");

    tag_b.wait_with_output()?;

    let tag_c = Command::new("git")
        .current_dir(&monorepo_temp_dir)
        .arg("tag")
        .arg("-a")
        .arg("@scope/package-c@1.0.0")
        .arg("-m")
        .arg("chore: release package-c@1.0.0")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Git tag problem");

    tag_c.wait_with_output()?;

    let canonic_path = &std::fs::canonicalize(Path::new(&monorepo_temp_dir)).unwrap();
    let root = canonic_path.as_path().display().to_string();

    Ok(PathBuf::from(root))
}

#[cfg(test)]
mod tests {
    //use super::*;

    #[test]
    fn test_parse_package_json() {
        let package_json = r#"
          {
            "name": "@websublime/workspace-tools",
            "version": "1.0.0",
            "description": "My package json",
            "main": "./dist/index.js",
            "module": "./dist/index.js",
            "exports": {
              ".": {
                "types": "./dist/index.d.ts",
                "default": "./dist/index.js"
              }
            },
            "typesVersions": {
              "*": {
                "index.d.ts": [
                  "./dist/index.d.ts"
                ]
              }
            },
            "scripts": {
              "type-check": "vue-tsc --noEmit",
              "test": "vitest run --coverage",
              "build": "vite build --config vite.config.mts --mode=production --emptyOutDir=true",
              "dev": "vite --config vite.config.mts --debug --force"
            },
            "keywords": [],
            "author": "",
            "license": "ISL",
            "publishConfig": {
              "scope": "@websublime"
            },
            "repository": {
              "url": "git+ssh://git@github.com/websublime/workspace-tools.git",
              "type": "git"
            },
            "dependencies": {
              "@websublime/workspace-tools": "^0.30.0"
            },
            "devDependencies": {
              "@types/jsdom": "^21.1.6",
              "@types/node": "^20.11.6",
              "@workbench/core": "^7.21.0",
              "@vitest/coverage-v8": "^2.0.4",
              "@vue/test-utils": "2.3.2",
              "happy-dom": "^14.12.3",
              "typescript": "^5.3.3",
              "@vitejs/plugin-vue": "^5.0.3",
              "vite": "^5.3.4",
              "vitest": "^2.0.4",
              "vue": "3.3.4",
              "vue-tsc": "^1.8.27"
            },
            "files": [
              "dist",
              "manifest.json",
              "./LICENSE.md",
              "./README.md",
              "./CHANGELOG.md"
            ]
          }"#;
        let package_json_parsed = serde_json::from_str::<serde_json::Value>(package_json).unwrap();

        assert_eq!(package_json_parsed.is_object(), true);
    }
}
