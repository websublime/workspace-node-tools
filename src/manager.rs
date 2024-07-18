//! # Package Manager
//!
//! This package ables to detect which package manager is being used in the monorepo.
#![allow(clippy::all)]
use std::{
    collections::HashMap, fmt::Display, fmt::Formatter, fmt::Result as FmtResult, path::Path,
};

#[cfg(feature = "napi")]
#[napi(string_enum)]
#[derive(Debug, PartialEq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl Display for PackageManager {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let package_manager = match self {
            PackageManager::Npm => "npm".to_string(),
            PackageManager::Yarn => "yarn".to_string(),
            PackageManager::Pnpm => "pnpm".to_string(),
            PackageManager::Bun => "bun".to_string(),
        };

        write!(f, "{}", package_manager)
    }
}

/// Detects which package manager is available in the workspace.
pub fn detect_package_manager(path: &Path) -> Option<PackageManager> {
    let package_manager_files = HashMap::from([
        ("package-lock.json", PackageManager::Npm),
        ("npm-shrinkwrap.json", PackageManager::Npm),
        ("yarn.lock", PackageManager::Yarn),
        ("pnpm-lock.yaml", PackageManager::Pnpm),
        ("bun.lockb", PackageManager::Bun),
    ]);

    for (file, package_manager) in package_manager_files.iter() {
        let lock_file = path.join(file);

        if lock_file.exists() {
            return Some(*package_manager);
        }
    }

    if let Some(parent) = path.parent() {
        return detect_package_manager(&parent);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{paths::get_project_root_path, utils::create_test_monorepo};
    use std::{fs::remove_dir_all, path::PathBuf};

    #[test]
    fn package_manager_for_npm_lock() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Npm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let package_manager =
            detect_package_manager(&PathBuf::from(project_root.unwrap()).as_path());

        assert_eq!(package_manager, Some(PackageManager::Npm));
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn package_manager_for_yarn_lock() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Yarn)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let package_manager =
            detect_package_manager(&PathBuf::from(project_root.unwrap()).as_path());

        assert_eq!(package_manager, Some(PackageManager::Yarn));
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn package_manager_for_pnpm_lock() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Pnpm)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let package_manager =
            detect_package_manager(&PathBuf::from(project_root.unwrap()).as_path());

        assert_eq!(package_manager, Some(PackageManager::Pnpm));
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn package_manager_for_bun_lock() -> Result<(), std::io::Error> {
        let ref monorepo_dir = create_test_monorepo(&PackageManager::Bun)?;
        let project_root = get_project_root_path(Some(monorepo_dir.to_path_buf()));

        let package_manager =
            detect_package_manager(&PathBuf::from(project_root.unwrap()).as_path());

        assert_eq!(package_manager, Some(PackageManager::Bun));
        remove_dir_all(&monorepo_dir)?;
        Ok(())
    }

    #[test]
    fn package_manager_not_present() {
        let path = std::env::current_dir().expect("Current user home directory");
        let package_manager = detect_package_manager(&path);

        assert_eq!(package_manager, None);
    }

    #[test]
    #[should_panic]
    fn package_manager_empty_display_should_panic() {
        let path = std::env::current_dir().expect("Current user home directory");
        let package_manager = detect_package_manager(&path);

        assert_eq!(package_manager.unwrap().to_string(), String::from(""));
    }
}
