use std::path::PathBuf;
// use napi::bindgen_prelude::*;
use napi::{Env, JsObject, Result as NapiResult};

use workspace_std::{changes::Changes, config::get_workspace_config};

#[napi(js_name = "initChanges")]
pub fn js_init_changes(env: Env, cwd: Option<String>) -> NapiResult<JsObject> {
    let mut changes_object = env.create_object().unwrap();

    let root = match cwd {
        Some(cwd) => Some(PathBuf::from(cwd)),
        None => None,
    };

    let ref config = get_workspace_config(root);
    let changes = Changes::from(config);

    let data = changes.init();
    data.changes.iter().for_each(|(key, _change)| {
        changes_object.set_named_property(key.as_str(), "test").unwrap();
    });

    Ok(changes_object)
}
