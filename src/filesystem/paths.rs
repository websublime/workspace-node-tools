use std::{env, path};

pub fn get_project_root_path() -> Option<String> {
	let current_dir = env::current_dir().unwrap();

	let dir = match walk_reverse_dir(current_dir.as_path()) {
		Some(path) => path,
		None => String::new(),
	};

	Some(dir)
}

fn walk_reverse_dir(path: &path::Path) -> Option<String> {
	let current_path = path.to_path_buf();
    let map_files = vec![
        ("package-lock.json", "npm"),
        ("npm-shrinkwrap.json", "npm"),
        ("yarn.lock", "yarn"),
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lockb", "bun"),
    ];

    for (file, _) in map_files.iter() {
        let lock_file = current_path.join(file);

        if lock_file.exists() {
            return Some(current_path.to_str().unwrap().to_string());
        }
    }

    if let Some(parent) = path.parent() {
        return walk_reverse_dir(parent);
    }

	None
}
