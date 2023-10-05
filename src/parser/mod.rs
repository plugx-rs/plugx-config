use crate::is_trace_level_enabled;
use cfg_if::cfg_if;
use plugx_input::Input;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
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

pub type BoxedModifierFn =
    Box<dyn Fn(&Input) -> anyhow::Result<Option<HashMap<String, Input>>> + Send + Sync>;
const MODIFIER_FN_DEBUG: &str = stringify!(
    Box<dyn Fn(&mut Input) -> Result<()>, ConfigurationParserError>>
);

#[derive(Debug, Error)]
pub enum ConfigurationParserError {
    #[error("{parser} with supported formats {supported_format_list:?} could not parse `{data}`")]
    Parse {
        data: String,
        parser: String,
        supported_format_list: Vec<String>,
        source: anyhow::Error,
    },
    #[error("Could not found parser")]
    ParserNotFound,
}

pub trait ConfigurationParser: Send + Sync + Debug + Display {
    fn maybe_get_modifier(&self) -> Option<&BoxedModifierFn> {
        None
    }

    fn supported_format_list(&self) -> Vec<String>;

    fn parse_and_modify_with_logging(
        &self,
        bytes: &[u8],
    ) -> Result<Input, ConfigurationParserError> {
        self.parse_with_logging(bytes).and_then(|mut input| {
            if let Some(modifier) = self.maybe_get_modifier() {
                let maybe_input_clone = if is_trace_level_enabled!() {
                    Some(input.clone())
                } else {
                    None
                };
                modifier(&mut input).map_err(|error| ConfigurationParserError::Parse {
                    data: String::from_utf8_lossy(bytes).to_string(),
                    parser: self.to_string(),
                    supported_format_list: self.supported_format_list().to_vec(),
                    source: error,
                })?;
                if let Some(old_input) = maybe_input_clone {
                    if input != old_input {
                        cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::trace!(
                                    parser = self.to_string(),
                                    supported_format_list = ?self.supported_format_list(),
                                    from = old_input.to_string(),
                                    to = input.to_string(),
                                    "Modified parsed contents"
                                );
                            } else if #[cfg(feature = "logging")] {
                                log::trace!(
                                    "parser=\"{self}\" supported_format_list={:?}, from={:?} to={:?} message=\"Modified parsed contents\"",
                                    self.supported_format_list(),
                                    old_input.to_string(),
                                    input.to_string(),
                                );
                            }
                        }
                    }
                }
            };
            Ok(input)
        })
    }

    fn parse_with_logging(&self, bytes: &[u8]) -> Result<Input, ConfigurationParserError> {
        self.parse(bytes)
            .map(|input| {
                if is_trace_level_enabled!() {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::trace!(
                                parser = self.to_string(),
                                supported_format_list = ?self.supported_format_list(),
                                from = String::from_utf8_lossy(bytes).to_string(),
                                to = input.to_string(),
                                "Parsed contents"
                            );
                        } else if #[cfg(feature = "logging")] {
                            log::trace!(
                                "parser=\"{self}\" supported_format_list={:?}, from={:?} to={:?} message=\"Parsed contents\"",
                                self.supported_format_list(),
                                String::from_utf8_lossy(bytes).to_string(),
                                input.to_string(),
                            );
                        }
                    }
                };
                input
            })
            .map_err(|error| ConfigurationParserError::Parse {
                data: String::from_utf8_lossy(bytes).to_string(),
                parser: self.to_string(),
                supported_format_list: self.supported_format_list().to_vec(),
                source: error,
            })
    }

    fn parse_and_modify(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        self.parse(bytes).and_then(|mut input| {
            if let Some(modifier) = self.maybe_get_modifier() {
                let _ = modifier(&mut input)?;
            };
            Ok(input)
        })
    }

    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Input>;

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool>;
}
