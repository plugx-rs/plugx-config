use crate::parser::ConfigurationParser;
use anyhow::anyhow;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ConfigurationParserJson;

impl Default for ConfigurationParserJson {
    fn default() -> Self {
        Self
    }
}

impl ConfigurationParserJson {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Display for ConfigurationParserJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON")
    }
}

impl ConfigurationParser for ConfigurationParserJson {
    fn supported_format_list(&self) -> Vec<String> {
        ["json".into()].into()
    }

    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        serde_json::from_slice(bytes).map_err(|error| anyhow!(error))
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(serde_json::from_slice::<serde_json::Value>(bytes).is_ok())
    }
}
