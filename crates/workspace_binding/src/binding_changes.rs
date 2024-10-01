use napi::bindgen_prelude::Object;
use napi::Env;
use std::path::PathBuf;

use workspace_std::{changes::Changes, config::get_workspace_config};

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
