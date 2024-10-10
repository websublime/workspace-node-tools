use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;

use crate::dependency::Node;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct Dependency {
    name: String,
    version: VersionReq,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct Package {
    name: String,
    version: Version,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PackageInfo {
    pub package: Package,
    pub package_json_path: String,
    pub package_path: String,
    pub package_relative_path: String,
    pub pkg_json: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageJson {
    pub workspaces: Option<Vec<String>>,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub private: Option<bool>,
    pub license: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<Value>,
    pub dependencies: Option<Value>,
    pub dev_dependencies: Option<Value>,
    pub peer_dependencies: Option<Value>,
    pub optional_dependencies: Option<Value>,
    pub engines: Option<Value>,
    pub scripts: Option<Value>,
    pub bin: Option<Value>,
}

impl Node for PackageInfo {
    type DependencyType = Dependency;

    fn dependencies(&self) -> &[Self::DependencyType] {
        &self.package.dependencies[..]
    }

    fn matches(&self, dependency: &Self::DependencyType) -> bool {
        let dependency_version =
            semver::VersionReq::parse(&dependency.version.to_string()).unwrap();
        let self_version = semver::Version::parse(&self.package.version.to_string()).unwrap();

        // Check that name is an exact match, and that the dependency
        // requirements are fulfilled by our own version
        self.package.name == dependency.name && dependency_version.matches(&self_version)
    }
}

impl Node for Package {
    type DependencyType = Dependency;

    fn dependencies(&self) -> &[Self::DependencyType] {
        &self.dependencies[..]
    }

    fn matches(&self, dependency: &Self::DependencyType) -> bool {
        let dependency_version =
            semver::VersionReq::parse(&dependency.version.to_string()).unwrap();
        let self_version = semver::Version::parse(&self.version.to_string()).unwrap();

        // Check that name is an exact match, and that the dependency
        // requirements are fulfilled by our own version
        self.name == dependency.name && dependency_version.matches(&self_version)
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

#[cfg(test)]
#[allow(clippy::print_stdout)]
#[allow(clippy::uninlined_format_args)]
mod tests {
    use super::*;
    use crate::dependency::DependencyGraph;
    use petgraph::dot::Dot;
    use semver::Version;

    fn build_packages() -> Vec<Package> {
        vec![
            Package {
                name: "@scope/bar".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![Dependency {
                    name: "@scope/foo".to_string(),
                    version: ">=2.0.0".parse().unwrap(),
                }],
            },
            Package {
                name: "@scope/foo".to_string(),
                version: Version::parse("2.0.0").unwrap(),
                dependencies: vec![],
            },
            Package {
                name: "@scope/baz".to_string(),
                version: Version::parse("3.0.0").unwrap(),
                dependencies: vec![
                    Dependency {
                        name: "@scope/bar".to_string(),
                        version: ">=1.0.0".parse().unwrap(),
                    },
                    Dependency {
                        name: "@scope/foo".to_string(),
                        version: ">=2.0.0".parse().unwrap(),
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_display() {
        let pkgs = build_packages();
        let dependency_graph = DependencyGraph::from(&pkgs[..]);
        let dot = Dot::new(&dependency_graph.graph);
        println!("{:?}", dot);
    }
}
