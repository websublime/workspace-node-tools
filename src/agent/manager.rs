use std::{collections::HashMap, fmt::Display, fmt::Formatter, fmt::Result, path::Path};

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

    /*pub fn to_string(&self) -> String {
    match self {
        Agent::Npm => "npm".to_string(),
        Agent::Yarn => "yarn".to_string(),
        Agent::Pnpm => "pnpm".to_string(),
        Agent::Bun => "bun".to_string(),
    }
    }*/
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

    #[test]
    fn agent_not_present() {
        let path = std::env::current_dir().expect("Current user home directory");
        let agent = Agent::detect(&path);

        dbg!(path);

        assert_eq!(agent, None);
    }
}