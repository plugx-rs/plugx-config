//! Configuration parser trait and implementations.

use plugx_input::Input;
use std::fmt::Debug;
use thiserror::Error;

#[cfg(feature = "env")]
pub mod env;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "toml")]
pub mod toml;
#[cfg(feature = "yaml")]
pub mod yaml;

pub mod closure;

/// Parser error type.
#[derive(Debug, Error)]
pub enum ConfigurationParserError {
    /// Could not parse contents.
    #[error("{parser} with supported formats {supported_format_list:?} could not parse `{data}`")]
    Parse {
        data: String,
        parser: String,
        supported_format_list: Vec<String>,
        source: anyhow::Error,
    },
    /// Could not found parser or guess format to choose correct parser.
    #[error("Could not found parser to parse format `{format}`")]
    ParserNotFound { format: String },
}

/// A trait to parse configuration contents.
pub trait ConfigurationParser: Send + Sync + Debug {
    /// Name of this parser (e.g. "YAML")
    fn name(&self) -> String;

    /// Supported format list (e.g. "yml")
    fn supported_format_list(&self) -> Vec<String>;

    /// Parses a byte slice to [Input].
    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input>;

    /// Checks if provided byte slice is ok for future parsing. (e.g. is it YAML at all or not)
    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool>;

    fn parse(&self, bytes: &[u8]) -> Result<Input, ConfigurationParserError> {
        self.try_parse(bytes)
            .map_err(|source| ConfigurationParserError::Parse {
                data: String::from_utf8_lossy(bytes).to_string(),
                parser: self.name(),
                supported_format_list: self.supported_format_list(),
                source,
            })
    }
}
