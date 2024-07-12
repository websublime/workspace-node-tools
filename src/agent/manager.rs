use std::{collections::HashMap, fmt::Display, fmt::Formatter, fmt::Result, path::Path};

#[cfg(feature = "napi")]
#[napi(string_enum)]
#[derive(Debug, PartialEq)]
pub enum Agent {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

#[cfg(not(feature = "napi"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Agent {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl Agent {
    pub fn detect(path: &Path) -> Option<Agent> {
        let agent_files = HashMap::from([
            ("package-lock.json", Agent::Npm),
            ("npm-shrinkwrap.json", Agent::Npm),
            ("yarn.lock", Agent::Yarn),
            ("pnpm-lock.yaml", Agent::Pnpm),
            ("bun.lockb", Agent::Bun),
        ]);

        for (file, agent) in agent_files.iter() {
            let lock_file = path.join(file);

            if lock_file.exists() {
                return Some(*agent);
            }
        }

        if let Some(parent) = path.parent() {
            return Self::detect(parent);
        }

        None
    }
}

impl Display for Agent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let agent = match self {
            Agent::Npm => "npm".to_string(),
            Agent::Yarn => "yarn".to_string(),
            Agent::Pnpm => "pnpm".to_string(),
            Agent::Bun => "bun".to_string(),
        };

        write!(f, "{}", agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, File};

    fn create_agent_file(path: &Path) -> File {
        File::create(path).expect("File not created")
    }

    fn delete_agent_file(path: &Path) {
        remove_file(path).expect("File not deleted");
    }

    #[test]
    fn agent_for_npm_lock() {
        let path = std::env::current_dir().expect("Current user home directory");
        let npm_lock = path.join("package-lock.json");

        create_agent_file(&npm_lock);

        let agent = Agent::detect(&path);

        assert_eq!(agent, Some(Agent::Npm));

        delete_agent_file(&npm_lock);
    }

    #[test]
    fn agent_for_yarn_lock() {
        let path = std::env::current_dir().expect("Current user home directory");
        let yarn_lock = path.join("yarn.lock");

        create_agent_file(&yarn_lock);

        let agent = Agent::detect(&path);

        assert_eq!(agent, Some(Agent::Yarn));

        delete_agent_file(&yarn_lock);
    }

    #[test]
    fn agent_for_pnpm_lock() {
        let path = std::env::current_dir().expect("Current user home directory");
        let pnpm_lock = path.join("pnpm-lock.yaml");

        create_agent_file(&pnpm_lock);

        let agent = Agent::detect(&path);

        assert_eq!(agent, Some(Agent::Pnpm));

        delete_agent_file(&pnpm_lock);
    }

    #[test]
    fn agent_for_bun_lock() {
        let path = std::env::current_dir().expect("Current user home directory");
        let bun_lock = path.join("bun.lockb");

        create_agent_file(&bun_lock);

        let agent = Agent::detect(&path);

        assert_eq!(agent, Some(Agent::Bun));

        delete_agent_file(&bun_lock);
    }

    #[test]
    fn agent_not_present() {
        let path = std::env::current_dir().expect("Current user home directory");
        let agent = Agent::detect(&path);

        assert_eq!(agent, None);
    }

    #[test]
    #[should_panic]
    fn agent_empty_display_should_panic() {
        let path = std::env::current_dir().expect("Current user home directory");
        let agent = Agent::detect(&path);

        assert_eq!(agent.unwrap().to_string(), String::from(""));
    }
}
