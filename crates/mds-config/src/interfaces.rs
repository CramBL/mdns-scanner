use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{config_type::ConfigType, error::ConfigLoadError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interfaces {
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    pub include_docker: bool,
    #[serde(skip)]
    compiled_ignore_patterns: Option<Vec<Regex>>, // Cached compiled regexes
    #[serde(skip)]
    compiled_patterns_hash: Option<u64>,
}

impl Default for Interfaces {
    fn default() -> Self {
        Self {
            ignore_patterns: mds_default::INTERFACES_IGNORE_PATTERNS
                .value
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            include_docker: mds_default::INTERFACES_INCLUDE_DOCKER.value,
            compiled_ignore_patterns: None,
            compiled_patterns_hash: None,
        }
    }
}

impl Interfaces {
    pub fn items(&mut self) -> Vec<ConfigType> {
        vec![
            ConfigType::RegexStringList {
                key: "Ignore Patterns",
                val: &mut self.ignore_patterns,
                description: mds_default::INTERFACES_IGNORE_PATTERNS.description,
            },
            ConfigType::Toggle {
                key: "Include Docker",
                val: &mut self.include_docker,
                description: mds_default::INTERFACES_INCLUDE_DOCKER.description,
            },
        ]
    }

    /// Calculates a hash of the current ignore_patterns.
    fn calculate_patterns_hash(patterns: &[String]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for pattern in patterns {
            pattern.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Retrieves compiled regex patterns, recompiling if source patterns have changed.
    /// Panics if regex compilation fails.
    pub fn ignore_patterns(&mut self) -> &[Regex] {
        let current_patterns_hash = Self::calculate_patterns_hash(&self.ignore_patterns);

        if self.compiled_ignore_patterns.is_none()
            || (self.compiled_patterns_hash != Some(current_patterns_hash))
        {
            self.compile_ignore_patterns()
                .expect("Failed to compile interface ignore regex patterns. Check config validity for 'ignore_patterns'.");
        }

        self.compiled_ignore_patterns
            .as_ref()
            .expect("compiled_ignore_patterns should be populated after successful compilation")
    }

    pub fn include_docker(&self) -> bool {
        self.include_docker
    }

    /// Compiles `ignore_patterns` into `Regex` objects and caches them.
    pub fn compile_ignore_patterns(&mut self) -> Result<(), ConfigLoadError> {
        let compiled_regexes: Vec<Regex> = self
            .ignore_patterns
            .iter()
            // Ensure we don't use an empty string which would filter everything
            .filter(|s| !s.is_empty())
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<Regex>, regex::Error>>()?;

        self.compiled_ignore_patterns = Some(compiled_regexes);
        self.compiled_patterns_hash = Some(Self::calculate_patterns_hash(&self.ignore_patterns));
        Ok(())
    }
}

impl PartialEq for Interfaces {
    fn eq(&self, other: &Self) -> bool {
        let Interfaces {
            ignore_patterns,
            include_docker,
            compiled_ignore_patterns: _,
            compiled_patterns_hash: _,
        } = self;

        let Interfaces {
            ignore_patterns: other_ignore_patterns,
            include_docker: other_include_docker,
            compiled_ignore_patterns: _,
            compiled_patterns_hash: _,
        } = other;

        ignore_patterns == other_ignore_patterns && include_docker == other_include_docker
    }
}
