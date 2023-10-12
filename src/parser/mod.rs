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

/// A modifier [Fn] that modifies parsed contents (if needed).
pub type BoxedModifierFn =
    Box<dyn Fn(&[u8], &mut Input) -> Result<(), ConfigurationParserError> + Send + Sync>;

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
    #[error("Could not found parser")]
    ParserNotFound,
}

/// A trait to parse configuration contents.
pub trait ConfigurationParser: Send + Sync + Debug {
    /// Supported format list (e.g. "yml")
    fn supported_format_list(&self) -> Vec<String>;

    /// Parses a byte slice to [Input].
    fn try_parse(&self, bytes: &[u8]) -> Result<Input, ConfigurationParserError>;

    /// Checks if provided byte slice is ok for future parsing.
    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool>;
}
