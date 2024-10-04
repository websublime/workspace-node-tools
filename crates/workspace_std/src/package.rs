use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dependency::Node;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct Package {
    name: String,
    version: String,
    dependencies: Vec<Package>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PackageInfo {
    pub package: Package,
    pub package_json_path: String,
    pub package_path: String,
    pub package_relative_path: String,
    pub pkg_json: Value,
}

impl Node for PackageInfo {
    type DependencyType = Package;

    fn dependencies(&self) -> &[Self::DependencyType] {
        &self.package.dependencies[..]
    }

    fn matches(&self, dependency: &Self::DependencyType) -> bool {
        let dependency_version = semver::VersionReq::parse(&dependency.version).unwrap();
        let self_version = semver::Version::parse(&self.package.version).unwrap();

        // Check that name is an exact match, and that the dependency
        // requirements are fulfilled by our own version
        self.package.name == dependency.name && dependency_version.matches(&self_version)
    }
}
