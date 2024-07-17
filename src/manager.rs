//! # Package Manager
//!
//! This package ables to detect which package manager is being used in the monorepo.

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

/*
#[cfg(test)]
mod tests {
    use super::*;
    use core::time;
    use std::{fs::{remove_file, File}, thread};

    fn create_package_manager_file(path: &Path) -> Result<File, std::io::Error> {
        let file = File::create(path)?;
        thread::sleep(time::Duration::from_secs(1));
        Ok(file)
    }

    fn delete_package_manager_file(path: &Path) -> Result<(), std::io::Error> {
        remove_file(path)?;
        thread::sleep(time::Duration::from_secs(1));
        Ok(())
    }

    #[test]
    fn package_manager_for_npm_lock() -> Result<(), std::io::Error> {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");

        create_package_manager_file(&npm_lock)?;

        let package_manager = detect_package_manager(&path);

        assert_eq!(package_manager, Some(PackageManager::Npm));

        delete_package_manager_file(&npm_lock)?;
        Ok(())
    }

    #[test]
    fn package_manager_for_yarn_lock() -> Result<(), std::io::Error> {
        let path = std::env::current_dir().expect("Current user home directory");
        let yarn_lock = path.join("yarn.lock");

        create_package_manager_file(&yarn_lock)?;

        let package_manager = detect_package_manager(&path);

        assert_eq!(package_manager, Some(PackageManager::Yarn));

        delete_package_manager_file(&yarn_lock)?;
        Ok(())
    }

    #[test]
    fn package_manager_for_pnpm_lock() -> Result<(), std::io::Error> {
        let path = std::env::current_dir().expect("Current user home directory");
        let pnpm_lock = path.join("pnpm-lock.yaml");

        create_package_manager_file(&pnpm_lock)?;

        let package_manager = detect_package_manager(&path);

        assert_eq!(package_manager, Some(PackageManager::Pnpm));

        delete_package_manager_file(&pnpm_lock)?;
        Ok(())
    }

    #[test]
    fn package_manager_for_bun_lock() -> Result<(), std::io::Error> {
        let path = std::env::current_dir().expect("Current user home directory");
        let bun_lock = path.join("bun.lockb");

        dbg!(&bun_lock);
        dbg!(&path);

        create_package_manager_file(&bun_lock)?;

        let package_manager = detect_package_manager(&path);

        assert_eq!(package_manager, Some(PackageManager::Bun));

        delete_package_manager_file(&bun_lock)?;
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
}*/
