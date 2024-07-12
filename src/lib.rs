//! # Workspace node tools
//!
//! This crate provides a set of tools to work with node workspaces.
//! It allows you to retrieve information about the workspace, and to interact with the workspace.
#[allow(unused_imports)]
#[macro_use]
#[cfg(feature = "napi-derive")]
extern crate napi_derive;

// # Agent
//
/// The agent module provides which package manager is available in the workspace.
pub mod agent;

// # Filesystem
//
/// The filesystem module provides utilities to work with the filesystem.
pub mod filesystem;

// # Git
//
/// The git module provides utilities to work with git.
pub mod git;

// # Monorepo
//
/// The monorepo module provides utilities to work with monorepos.
pub mod monorepo;
