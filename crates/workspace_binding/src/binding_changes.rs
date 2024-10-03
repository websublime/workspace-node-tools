use napi::{bindgen_prelude::Object, Error, Result};
use napi::{Env, Status};
use std::path::PathBuf;

use workspace_std::{
    changes::{Change, Changes},
    config::get_workspace_config,
};

pub enum ChangesError {
    InvalidPackageProperty,
    InvalidReleaseAsProperty,
    InvalidChange,
    NapiError(Error<Status>),
}

impl AsRef<str> for ChangesError {
    fn as_ref(&self) -> &str {
        match self {
            Self::InvalidPackageProperty => "Invalid package property",
            Self::InvalidReleaseAsProperty => "Invalid releaseAs property",
            Self::InvalidChange => "Invalid change",
            Self::NapiError(e) => e.status.as_ref(),
        }
    }
}

#[napi(js_name = "initChanges", ts_return_type = "Changes")]
pub fn js_init_changes(env: Env, cwd: Option<String>) -> Object {
    let mut changes_object = env.create_object().unwrap();

    let root = cwd.map(PathBuf::from);

    let config = &get_workspace_config(root);
    let changes = Changes::from(config);

    let data = changes.init();

    data.changes.iter().for_each(|(key, change)| {
        let value = serde_json::to_value(change).unwrap();
        changes_object.set(key.as_str(), value).unwrap();
    });

    changes_object
}

#[napi(
    js_name = "addChange",
    ts_args_type = "change: Change, deploy_envs?: string[], cwd?: string"
)]
pub fn js_add_change(
    change: Object,
    deploy_envs: Option<Vec<String>>,
    cwd: Option<String>,
) -> Result<bool, ChangesError> {
    let package_name = change.get_named_property::<String>("package").or_else(|_| {
        Err(Error::new(ChangesError::InvalidPackageProperty, "Failed to get package property"))
    })?;

    let release_as = change.get_named_property::<String>("releaseAs").or_else(|_| {
        Err(Error::new(ChangesError::InvalidReleaseAsProperty, "Failed to get releaseAs property"))
    })?;

    let envs = deploy_envs.unwrap_or_default();
    let change = &Change { package: package_name, release_as };
    let root = cwd.map(PathBuf::from);
    let config = &get_workspace_config(root);
    let changes = Changes::from(config);

    Ok(changes.add(change, Some(envs)))
}

#[napi(js_name = "removeChange", ts_args_type = "branch: string, cwd?: string")]
pub fn js_remove_change(branch: String, cwd: Option<String>) -> bool {
    let root = cwd.map(PathBuf::from);
    let config = &get_workspace_config(root);
    let changes = Changes::from(config);

    changes.remove(branch.as_str())
}
