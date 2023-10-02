use crate::parser::ConfigurationParser;
use anyhow::{anyhow, bail};
use cfg_if::cfg_if;
use plugx_input::{position, position::InputPosition, Input};
use std::fmt::{Debug, Display, Formatter};

pub const NAME: &str = "Environment-Variables";
pub const DEFAULT_KEY_SEPARATOR: &str = "__";

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ConfigurationParserEnv {
    prefix: String,
    key_separator: String,
}

impl Default for ConfigurationParserEnv {
    fn default() -> Self {
        Self {
            prefix: "".to_string(),
            key_separator: DEFAULT_KEY_SEPARATOR.to_string(),
        }
    }
}

impl Display for ConfigurationParserEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(NAME)
    }
}

impl ConfigurationParser for ConfigurationParserEnv {
    fn supported_format_list(&self) -> Vec<String> {
        ["env".into()].into()
    }

    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        let text = String::from_utf8(bytes.to_vec()).map_err(|error| anyhow!(error))?;
        let mut list = dotenv_parser::parse_dotenv(text.as_str())
            .map_err(|error| anyhow!(error))?
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
        println!("{list:?}");
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
        )?;
        Ok(map)
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(serde_json::from_slice::<serde_json::Value>(bytes).is_ok())
    }
}

impl ConfigurationParserEnv {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_prefix<P: AsRef<str>>(&mut self, prefix: P) {
        let (prefix, key_separator) =
            Self::fix(Some(prefix.as_ref()), Some(self.key_separator.as_str()));
        self.prefix = prefix;
        self.key_separator = key_separator;
    }

    pub fn with_prefix<P: AsRef<str>>(mut self, prefix: P) -> Self {
        self.set_prefix(prefix);
        self
    }

    pub fn set_key_separator<K: AsRef<str>>(&mut self, key_separator: K) {
        let (prefix, key_separator) =
            Self::fix(Some(self.prefix.as_str()), Some(key_separator.as_ref()));
        self.prefix = prefix;
        self.key_separator = key_separator;
    }

    pub fn with_key_separator<K: AsRef<str>>(mut self, key_separator: K) -> Self {
        self.set_key_separator(key_separator);
        self
    }

    fn fix(maybe_prefix: Option<&str>, maybe_key_separator: Option<&str>) -> (String, String) {
        let key_separator = if let Some(key_separator) = maybe_key_separator {
            let trimmed_key_separator = key_separator.trim().to_string();
            if key_separator != trimmed_key_separator {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::trace!(
                            parser = NAME.to_string(),
                            old = key_separator,
                            new = trimmed_key_separator,
                            "updated environment-variable key separator"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "parser={:?} old={:?} new={:?} message=\"updated environment-variable key separator\"",
                            NAME.to_string(),
                            key_separator,
                            trimmed_key_separator,
                        );
                    }
                }
            };
            trimmed_key_separator
        } else {
            DEFAULT_KEY_SEPARATOR.to_string()
        };
        let prefix = if let Some(prefix) = maybe_prefix {
            let mut trimmed_prefix = prefix.trim().to_string();
            if !key_separator.is_empty() && !trimmed_prefix.ends_with(&key_separator) {
                trimmed_prefix += key_separator.as_str();
            }
            if prefix != trimmed_prefix {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::trace!(
                            parser = NAME.to_string(),
                            old = prefix,
                            new = trimmed_prefix,
                            "updated environment-variable prefix"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "parser={:?}, old={:?} new={:?} message=\"updated environment-variable prefix\"",
                            NAME.to_string(),
                            prefix,
                            trimmed_prefix,
                        );
                    }
                }
            };
            trimmed_prefix
        } else {
            String::new()
        };
        (prefix, key_separator)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::enable_logging;

    #[test]
    fn deserialize() {
        enable_logging();

        // let env = r#"
        // A__B__C=value # comment
        // X__Y__Z="qouted" # comment
        // this__is__a__list=[1,2,3,4,5,6,7] # with comment
        // and__this__is__a__map='{"hello", "world"}' # with comment
        // boolean=false
        // error__=oops
        // "#;
        // let f = InputFormatEnv::new().unwrap();
        // println!("{f:?}");
        // println!("{}", f.deserialize(env.as_bytes()).unwrap());
    }
}
