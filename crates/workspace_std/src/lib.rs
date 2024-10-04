//! workspace_std is a collection of utilities to help you work with the filesystem and git repositories.
//!
//! # Table of Contents
//!
//! 1. [High-level features](#high-level-features)
//! 2. [Git module](#git-module)
//!
//! # High-level features
//!
//! - Git wrapper
//! - Filesystem utilities
//! - Node package manager detection
//! - Test utilities
pub mod changes;
pub mod config;
pub mod dependency;
pub mod errors;
pub mod git;
pub mod manager;
pub mod package;
pub mod paths;
pub mod test;
pub mod types;
pub mod utils;
