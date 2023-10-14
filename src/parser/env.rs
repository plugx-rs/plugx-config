//! Environment-Variable configuration parser.
//!
//! This is only usable if you enabled `env` Cargo feature.
//!
//! ### Example
//! ```rust
//! use plugx_config::parser::{ConfigurationParser, env::ConfigurationParserEnv};
//! use plugx_input::Input;
//!
//! let bytes = br#"
//! FOO__BAR__BAZ=Qux
//! FOO__BAR__ABC=3.14 # Comments are supported!
//! FOO__XYZ=false
//! HELLO='["w", "o", "l", "d"]' # A JSON list
//! "#;
//!
//! let parser = ConfigurationParserEnv::new();
//! // You can set nested key separator like this:
//! // parser.set_key_separator("__");
//! let parsed: Input = parser.try_parse(bytes.as_slice()).unwrap();
//! assert!(
//!     parsed.map_ref().unwrap().len() == 2 &&
//!     parsed.map_ref().unwrap().contains_key("foo") &&
//!     parsed.map_ref().unwrap().contains_key("hello")
//! );
//! let foo = parsed.map_ref().unwrap().get("foo").unwrap();
//! assert!(
//!     foo.map_ref().unwrap().len() == 2 &&
//!     foo.map_ref().unwrap().contains_key("bar") &&
//!     foo.map_ref().unwrap().contains_key("xyz")
//! );
//! let bar = foo.map_ref().unwrap().get("bar").unwrap();
//! assert_eq!(bar.map_ref().unwrap().get("baz").unwrap(), &"Qux".into());
//! assert_eq!(bar.map_ref().unwrap().get("abc").unwrap(), &3.14.into());
//! let xyz = foo.map_ref().unwrap().get("xyz").unwrap();
//! assert_eq!(xyz, &false.into());
//! let list = ["w", "o", "l", "d"].into();
//! assert_eq!(parsed.map_ref().unwrap().get("hello").unwrap(), &list);
//! ```
//!

use crate::{
    error::ConfigurationParserError,
    parser::{BoxedModifierFn, ConfigurationParser},
};
use anyhow::{anyhow, bail};
use plugx_input::{position, position::InputPosition, Input};
use std::fmt::{Debug, Display, Formatter};

pub const NAME: &str = "Environment-Variables";
const SUPPORTED_FORMAT_LIST: &[&str] = &["env"];
pub const DEFAULT_KEY_SEPARATOR: &str = "__";

pub struct ConfigurationParserEnv {
    prefix: String,
    key_separator: String,
    maybe_modifier: Option<BoxedModifierFn>,
}

impl Default for ConfigurationParserEnv {
    fn default() -> Self {
        Self {
            prefix: Default::default(),
            key_separator: DEFAULT_KEY_SEPARATOR.to_string(),
            maybe_modifier: Default::default(),
        }
    }
}

impl Debug for ConfigurationParserEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationParserEnv")
            .field("prefix", &self.prefix)
            .field("key_separator", &self.key_separator)
            .finish()
    }
}

impl Display for ConfigurationParserEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(NAME)
    }
}

impl ConfigurationParser for ConfigurationParserEnv {
    fn supported_format_list(&self) -> Vec<String> {
        SUPPORTED_FORMAT_LIST
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    fn try_parse(&self, bytes: &[u8]) -> Result<Input, ConfigurationParserError> {
        let text =
            String::from_utf8(bytes.to_vec()).map_err(|error| ConfigurationParserError::Parse {
                data: String::from_utf8_lossy(bytes).to_string(),
                parser: NAME.to_string(),
                supported_format_list: self.supported_format_list(),
                source: error.into(),
            })?;
        let mut list = dotenv_parser::parse_dotenv(text.as_str())
            .map_err(|error| ConfigurationParserError::Parse {
                data: text.clone(),
                parser: NAME.to_string(),
                supported_format_list: self.supported_format_list(),
                source: anyhow!(error),
            })?
            .into_iter()
            .filter_map(|(key, value)| {
                if self.prefix.is_empty() {
                    Some((key, value))
                } else if key.starts_with(&self.prefix) {
                    Some((key.chars().skip(self.prefix.len()).collect(), value))
                } else {
                    None
                }
            })
            .collect::<Vec<(String, String)>>();
        list.sort_by_key(|(key, _)| key.to_string());

        let mut map = Input::new_map();
        update_input_from_env(
            &mut map,
            list.into_iter()
                .map(|(key, value)| {
                    (
                        key.split(self.key_separator.as_str())
                            .map(|key| key.to_lowercase())
                            .collect::<Vec<String>>(),
                        value,
                    )
                })
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .map_err(|error| ConfigurationParserError::Parse {
            data: text,
            parser: NAME.to_string(),
            supported_format_list: self.supported_format_list(),
            source: error,
        })?;
        if let Some(ref modifier) = self.maybe_modifier {
            modifier(bytes, &mut map)?;
        }
        Ok(map)
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
            Some(dotenv_parser::parse_dotenv(text.as_str()).is_ok())
        } else {
            Some(false)
        }
    }
}

impl ConfigurationParserEnv {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_prefix<P: AsRef<str>>(&mut self, prefix: P) {
        self.prefix = prefix.as_ref().to_string();
    }

    pub fn with_prefix<P: AsRef<str>>(mut self, prefix: P) -> Self {
        self.set_prefix(prefix);
        self
    }

    pub fn set_key_separator<K: AsRef<str>>(&mut self, key_separator: K) {
        self.key_separator = key_separator.as_ref().to_string();
    }

    pub fn with_key_separator<K: AsRef<str>>(mut self, key_separator: K) -> Self {
        self.set_key_separator(key_separator);
        self
    }

    pub fn set_modifier(&mut self, modifier: BoxedModifierFn) {
        self.maybe_modifier = Some(modifier);
    }

    pub fn with_modifier(mut self, modifier: BoxedModifierFn) -> Self {
        self.set_modifier(modifier);
        self
    }
}

fn update_input_from_env(
    input: &mut Input,
    env_list: &[(Vec<String>, String)],
) -> anyhow::Result<()> {
    for (key_list, value) in env_list {
        if key_list.is_empty() || key_list.iter().any(|key| key.is_empty()) {
            continue;
        }
        update_input_from_key_list(input, key_list, value.clone(), position::new())?;
    }
    Ok(())
}

fn update_input_from_key_list(
    input: &mut Input,
    key_list: &[String],
    value: String,
    position: InputPosition,
) -> anyhow::Result<()> {
    if key_list.len() == 1 {
        let value = if let Ok(value) = serde_json::from_str::<Input>(value.as_str()) {
            value
        } else {
            Input::from(value.clone())
        };
        let key = key_list[0].clone();
        input.map_mut().unwrap().insert(key, value);
        Ok(())
    } else {
        let (key, key_list) = key_list.split_first().unwrap();
        let position = position.new_with_key(key);
        if !input.map_ref().unwrap().contains_key(key) {
            input
                .map_mut()
                .unwrap()
                .insert(key.clone(), Input::new_map());
        }
        let inner_input = input.map_mut().unwrap().get_mut(key).unwrap();
        if inner_input.is_map() {
            update_input_from_key_list(inner_input, key_list, value, position)
        } else {
            bail!(
                "{position} already exists with type {}, but we needed {}",
                inner_input.type_name(),
                Input::map_type_name()
            )
        }
    }
}
