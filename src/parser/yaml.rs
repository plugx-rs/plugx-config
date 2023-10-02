use crate::parser::ConfigurationParser;
use anyhow::anyhow;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ConfigurationParserYaml;

impl Default for ConfigurationParserYaml {
    fn default() -> Self {
        Self
    }
}

impl ConfigurationParserYaml {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Display for ConfigurationParserYaml {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("YAML")
    }
}

impl ConfigurationParser for ConfigurationParserYaml {
    fn supported_format_list(&self) -> Vec<String> {
        ["yml".into(), "yaml".into()].into()
    }

    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        serde_yaml::from_slice(bytes).map_err(|error| anyhow!(error))
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(serde_yaml::from_slice::<serde_yaml::Value>(bytes).is_ok())
    }
}
