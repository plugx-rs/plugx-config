use crate::parser::ConfigurationParser;
use anyhow::anyhow;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ConfigurationParserToml;

impl Default for ConfigurationParserToml {
    fn default() -> Self {
        Self
    }
}

impl ConfigurationParserToml {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Display for ConfigurationParserToml {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TOML")
    }
}

impl ConfigurationParser for ConfigurationParserToml {
    fn supported_format_list(&self) -> Vec<String> {
        ["toml".into()].into()
    }

    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        String::from_utf8(bytes.to_vec())
            .map_err(|error| anyhow!(error))
            .and_then(|text| toml::from_str(text.as_str()).map_err(|error| anyhow!(error)))
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(if let Ok(text) = String::from_utf8(bytes.to_vec()) {
            toml::from_str::<toml::Value>(text.as_str()).is_ok()
        } else {
            false
        })
    }
}
