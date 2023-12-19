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
//! let parsed: Input = parser.parse(bytes.as_slice()).unwrap();
//! assert!(
//!     parsed.as_map().len() == 2 &&
//!     parsed.as_map().contains_key("foo") &&
//!     parsed.as_map().contains_key("hello")
//! );
//! let foo = parsed.as_map().get("foo").unwrap();
//! assert!(
//!     foo.as_map().len() == 2 &&
//!     foo.as_map().contains_key("bar") &&
//!     foo.as_map().contains_key("xyz")
//! );
//! let bar = foo.as_map().get("bar").unwrap();
//! assert_eq!(bar.as_map().get("baz").unwrap(), &"Qux".into());
//! assert_eq!(bar.as_map().get("abc").unwrap(), &3.14.into());
//! let xyz = foo.as_map().get("xyz").unwrap();
//! assert_eq!(xyz, &false.into());
//! let list = ["w", "o", "l", "d"].into();
//! assert_eq!(parsed.as_map().get("hello").unwrap(), &list);
//! ```
//!

use crate::parser::ConfigurationParser;
use anyhow::{anyhow, bail};
use plugx_input::{position, position::InputPosition, Input};
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone)]
pub struct ConfigurationParserEnv {
    separator: String,
}

impl Default for ConfigurationParserEnv {
    fn default() -> Self {
        Self {
            separator: crate::loader::env::default::option::separator(),
        }
    }
}

impl Display for ConfigurationParserEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Environment-Variables parser")
    }
}

impl ConfigurationParser for ConfigurationParserEnv {
    fn name(&self) -> String {
        "Environment-Variables".to_string()
    }

    fn supported_format_list(&self) -> Vec<String> {
        ["env".into()].into()
    }

    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|error| anyhow!("Could not decode contents to UTF-8 ({error})"))?;
        let mut list = dotenv_parser::parse_dotenv(text.as_str())
            .map_err(|error| anyhow!(error))?
            .into_iter()
            .collect::<Vec<(String, String)>>();
        list.sort_by_key(|(key, _)| key.to_string());

        let mut map = Input::new_map();
        update_input_from_env(
            &mut map,
            list.into_iter()
                .map(|(key, value)| {
                    (
                        key.split(self.separator.as_str())
                            .map(|key| key.to_lowercase())
                            .collect::<Vec<String>>(),
                        value,
                    )
                })
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .map_err(|error| anyhow!(error))?;
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

    pub fn set_key_separator<K: AsRef<str>>(&mut self, key_separator: K) {
        self.separator = key_separator.as_ref().to_string();
    }

    pub fn with_key_separator<K: AsRef<str>>(mut self, key_separator: K) -> Self {
        self.set_key_separator(key_separator);
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
        input.map_mut().insert(key, value);
        Ok(())
    } else {
        let (key, key_list) = key_list.split_first().unwrap();
        let position = position.new_with_key(key);
        if !input.as_map().contains_key(key) {
            input.map_mut().insert(key.clone(), Input::new_map());
        }
        let inner_input = input.map_mut().get_mut(key).unwrap();
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
