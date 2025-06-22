use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::ConfigLoadError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interfaces {
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    pub include_docker: bool,
    #[serde(skip)]
    pub compiled_ignore_patterns: Option<Vec<Regex>>, // Cached compiled regexes
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
        }
    }
}

impl Interfaces {
    /// Get compiled regex patterns for interface ignoring from the cache.
    /// Panics if called before config is loaded and regexes are compiled.
    pub fn ignore_patterns(&self) -> &[Regex] {
        self.compiled_ignore_patterns
            .as_ref()
            .expect("ignore_patterns called before AppConfig was fully loaded and compiled.")
    }

    pub fn include_docker(&self) -> bool {
        self.include_docker
    }

    pub fn compile_ignore_patterns(&mut self) -> Result<(), ConfigLoadError> {
        let compiled_regexes: Vec<Regex> = self
            .ignore_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<Regex>, regex::Error>>()?;
        self.compiled_ignore_patterns = Some(compiled_regexes);
        Ok(())
    }
}

impl PartialEq for Interfaces {
    fn eq(&self, other: &Self) -> bool {
        let Interfaces {
            ignore_patterns,
            include_docker,
            compiled_ignore_patterns: _,
        } = self;

        let Interfaces {
            ignore_patterns: other_ignore_patterns,
            include_docker: other_include_docker,
            compiled_ignore_patterns: _,
        } = other;

        ignore_patterns == other_ignore_patterns && include_docker == other_include_docker
    }
}
