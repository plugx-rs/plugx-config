//! Environment-Variables configuration loader.
//!
//! This is only usable if you enabled `env` Cargo feature.
//!
//! ### Example
//! ```rust
//! use std::collections::HashMap;
//! use std::env::set_var;
//! use url::Url;
//! use plugx_config::loader::{ConfigurationLoader, env::ConfigurationLoaderEnv};
//!
//! set_var("MY_APP_NAME__FOO__B_A_R", "Baz");
//! set_var("MY_APP_NAME__QUX__ABC", "XYZ");
//!
//! let url = Url::try_from("env://?prefix=MY_APP_NAME").expect("A valid URL!");
//!
//! let mut loader = ConfigurationLoaderEnv::new();
//! // You could set `prefix` and `separator` like this too:
//! // loader.set_prefix("MY_APP_NAME");
//! // loader.set_separator("__");
//!
//! // We do not set `whitelist` so we're going to load all plugins configurations:
//! let mut maybe_whitelist = None;
//! let result = loader.try_load(&url, maybe_whitelist).unwrap();
//! let (_, foo) = result.iter().find(|(plugin_name, _)| plugin_name == "foo").expect("`foo` plugin config");
//! assert_eq!(foo.maybe_contents(), Some(&"B_A_R=\"Baz\"".to_string()));
//! let (_, qux) = result.iter().find(|(plugin_name, _)| plugin_name == "qux").expect("`qux` plugin config");
//! assert_eq!(qux.maybe_contents(), Some(&"ABC=\"XYZ\"".to_string()));
//!
//! // Only load (and not modify) `foo` plugin configurations:
//! let whitelist = ["foo".to_string()].to_vec();
//! maybe_whitelist = Some(whitelist.as_slice());
//! let result = loader.try_load(&url, maybe_whitelist).unwrap();
//! assert!(result.iter().find(|(plugin_name, _)| plugin_name == "foo").is_some());
//! assert!(result.iter().find(|(plugin_name, _)| plugin_name == "qux").is_none());
//! ```
//!
//! See [loader] documentation to known how loaders work.

use crate::{
    entity::ConfigurationEntity,
    loader::{self, ConfigurationLoadError, ConfigurationLoader},
};
use serde::Deserialize;
use std::{collections::HashMap, env, fmt::Debug};
use url::Url;
pub const NAME: &str = "Environment-Variables";
pub const SCHEME_LIST: &[&str] = &["env"];

/// Loads configurations from Environment-Variables.
#[derive(Debug, Default, Clone)]
pub struct ConfigurationLoaderEnv {
    options: ConfigurationLoaderEnvOptions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct ConfigurationLoaderEnvOptions {
    #[serde(rename = "prefix")]
    prefix: String,
    #[serde(rename = "separator")]
    separator: String,
    #[serde(rename = "strip_prefix")]
    strip_prefix: bool,
}

impl Default for ConfigurationLoaderEnvOptions {
    fn default() -> Self {
        Self {
            prefix: default::option::prefix(),
            separator: default::option::separator(),
            strip_prefix: default::option::strip_prefix(),
        }
    }
}

// impl Debug for ConfigurationLoaderEnv {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("ConfigurationLoaderEnv")
//             .field("options", &self.options)
//             .finish()
//     }
// }

pub mod default {
    pub mod option {
        pub fn prefix() -> String {
            let mut prefix = option_env!("CARGO_BIN_NAME").unwrap_or("").to_string();
            if prefix.is_empty() {
                prefix = option_env!("CARGO_CRATE_NAME").unwrap_or("").to_string();
            }
            if !prefix.is_empty() {
                prefix += separator().as_str();
            }
            prefix
        }

        pub fn separator() -> String {
            "__".to_string()
        }

        pub fn strip_prefix() -> bool {
            true
        }
    }
}

impl ConfigurationLoaderEnv {
    /// Same as `default()` method.
    pub fn new() -> Self {
        Default::default()
    }

    /// Only loads keys with this prefix.
    pub fn set_prefix<P: AsRef<str>>(&mut self, prefix: P) {
        self.options.prefix = prefix.as_ref().to_string();
    }

    /// Only loads keys with this prefix.
    pub fn with_prefix<P: AsRef<str>>(mut self, prefix: P) -> Self {
        self.set_prefix(prefix);
        self
    }

    /// Used is separating plugin names.
    pub fn set_separator<S: AsRef<str>>(&mut self, separator: S) {
        self.options.separator = separator.as_ref().to_string();
    }

    /// Used is separating plugin names.
    pub fn with_separator<S: AsRef<str>>(mut self, separator: S) -> Self {
        self.set_separator(separator);
        self
    }
}

impl ConfigurationLoader for ConfigurationLoaderEnv {
    fn name(&self) -> String {
        NAME.into()
    }

    /// In this case `["env"]`.
    fn scheme_list(&self) -> Vec<String> {
        SCHEME_LIST.iter().cloned().map(String::from).collect()
    }

    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<Vec<(String, ConfigurationEntity)>, ConfigurationLoadError> {
        let ConfigurationLoaderEnvOptions {
            mut prefix,
            mut separator,
            mut strip_prefix,
        } = loader::deserialize_query_string(NAME, url)?;
        if self.options.prefix != default::option::prefix() {
            prefix = self.options.prefix.clone()
        }
        if self.options.separator != default::option::separator() {
            separator = self.options.separator.clone()
        }
        if self.options.strip_prefix != default::option::strip_prefix() {
            strip_prefix = self.options.strip_prefix
        }
        if !separator.is_empty() && !prefix.is_empty() && !prefix.ends_with(separator.as_str()) {
            prefix += separator.as_str()
        }
        let mut result: HashMap<String, String> = HashMap::new();
        env::vars()
            .filter(|(key, _)| prefix.is_empty() || key.starts_with(prefix.as_str()))
            .map(|(mut key, value)| {
                if !prefix.is_empty() && strip_prefix {
                    key = key.chars().skip(prefix.chars().count()).collect::<String>()
                }
                (key, value)
            })
            .filter(|(key, _)| !key.is_empty())
            .map(|(key, value)| {
                let key_list = if separator.is_empty() {
                    [key].to_vec()
                } else {
                    key.splitn(2, separator.as_str())
                        .map(|key| key.to_string())
                        .collect()
                };
                (key_list, value)
            })
            .filter(|(key_list, _)| !key_list[0].is_empty())
            .map(|(mut key_list, value)| {
                let plugin_name = key_list.remove(0).to_lowercase();
                let key = if key_list.len() == 1 {
                    key_list.remove(0)
                } else {
                    String::new()
                };
                (plugin_name, key, value)
            })
            .filter(|(_, key, _)| !key.is_empty())
            .filter(|(plugin_name, _, _)| {
                maybe_whitelist
                    .as_ref()
                    .map(|whitelist| whitelist.contains(plugin_name))
                    .unwrap_or(true)
            })
            .for_each(|(plugin_name, key, value)| {
                let key_value = format!("{key}={value:?}");
                if let Some(configuration) = result.get_mut(&plugin_name) {
                    *configuration += "\n";
                    *configuration += key_value.as_str();
                } else {
                    result.insert(plugin_name, key_value);
                }
            });
        let result = result
            .into_iter()
            .map(|(plugin_name, contents)| {
                (
                    plugin_name.clone(),
                    ConfigurationEntity::new(url.clone(), plugin_name, NAME)
                        .with_format("env")
                        .with_contents(contents),
                )
            })
            .collect();
        Ok(result)
    }
}
