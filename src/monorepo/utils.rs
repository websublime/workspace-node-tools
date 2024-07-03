use regex::Regex;

#[derive(Debug)]
pub struct PackageScopeMetadata {
    pub full: String,
    pub name: String,
    pub version: String,
    pub path: Option<String>,
}

pub fn package_scope_name_version(pkg_name: &str) -> Option<PackageScopeMetadata> {
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
