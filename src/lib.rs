#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/2073802?s=200&v=4")]
//! # Workspace node tools
//!
//! This crate provides a set of tools to work with node workspaces.
//! It allows you to retrieve information about the workspace, and to interact with the workspace.
#[allow(unused_imports)]
#[macro_use]
#[cfg(feature = "napi-derive")]
extern crate napi_derive;

mod utils;

pub mod manager;

pub mod paths;

pub mod git;

pub mod packages;

pub mod conventional;

pub mod bumps;

pub mod changes;
