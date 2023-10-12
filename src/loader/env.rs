//! Configuration loader for Environment-Variables.
//!
//! This is only usable if you enabled `env` Cargo feature.
//!
//!
//! ```rust
//! use std::collections::HashMap;
//! use std::env::set_var;
//! use url::Url;
//! use plugx_config::loader::{ConfigurationLoader, env::ConfigurationLoaderEnv};
//!
//! set_var("MY_APP_NAME_FOO__B_A_R", "Baz");
//! set_var("MY_APP_NAME_QUX__ABC", "XYZ");
//!
//! let url = Url::try_from("env://?prefix=MY_APP_NAME_&key_separator=__").expect("A valid URL!");
//!
//! let mut loader = ConfigurationLoaderEnv::new();
//! // You could set `prefix` and `key_separator` like this too:
//! // loader.set_prefix("MY_APP_NAME_");
//! // loader.set_key_separator("__");
//!
//! let modifier = |url: &Url, loaded: &mut HashMap<String, _>| {
//!     // Modify loaded configuration if needed
//!     Ok(())
//! };
//! loader.set_modifier(Box::new(modifier)); // Note that setting a modifier is optional
//!
//! // We do not set `whitelist` so we're going to load all plugins configurations:
//! let mut maybe_whitelist = None;
//! let result = loader.try_load_and_maybe_modify(&url, maybe_whitelist).unwrap();
//! let foo = result.get("foo").unwrap();
//! assert_eq!(foo.maybe_contents(), Some(&"B_A_R=\"Baz\"".to_string()));
//! let qux = result.get("qux").unwrap();
//! assert_eq!(qux.maybe_contents(), Some(&"ABC=\"XYZ\"".to_string()));
//!
//! // Only load (and not modify) `foo` plugin configurations:
//! let whitelist = ["foo".to_string()].to_vec();
//! maybe_whitelist = Some(whitelist.as_slice());
//! let result = loader.try_load(&url, maybe_whitelist).unwrap();
//! assert!(result.get("foo").is_some());
//! assert!(result.get("qux").is_none());
//! ```

use crate::{
    entity::ConfigurationEntity,
    loader::{self, BoxedLoaderModifierFn, ConfigurationLoadError, ConfigurationLoader},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Formatter},
};
use url::Url;

const NAME: &str = "Environment-Variables";
const DEFAULT_KEY_SEPARATOR: &str = "__";

/// Loads configurations from Environment-Variables.
pub struct ConfigurationLoaderEnv {
    options: ConfigurationLoaderEnvOptions,
    maybe_modifier: Option<BoxedLoaderModifierFn>,
}

#[derive(Debug, Deserialize)]
struct ConfigurationLoaderEnvOptions {
    #[serde(rename = "prefix")]
    maybe_prefix: Option<String>,
    #[serde(rename = "key_separator")]
    maybe_key_separator: Option<String>,
}

impl Debug for ConfigurationLoaderEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationLoaderEnv")
            .field("options", &self.options)
            .finish()
    }
}

impl Default for ConfigurationLoaderEnvOptions {
    fn default() -> Self {
        Self {
            maybe_prefix: None,
            maybe_key_separator: None,
        }
    }
}

impl Default for ConfigurationLoaderEnv {
    fn default() -> Self {
        Self {
            options: Default::default(),
            maybe_modifier: Default::default(),
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
        self.options.maybe_prefix = Some(prefix.as_ref().to_string());
    }

    /// Only loads keys with this prefix.
    pub fn with_prefix<P: AsRef<str>>(mut self, prefix: P) -> Self {
        self.set_prefix(prefix);
        self
    }

    /// Used is separating plugin names.
    pub fn set_key_separator<K: AsRef<str>>(&mut self, key_separator: K) {
        self.options.maybe_key_separator = Some(key_separator.as_ref().to_string());
    }

    /// Used is separating plugin names.
    pub fn with_key_separator<K: AsRef<str>>(mut self, key_separator: K) -> Self {
        self.set_key_separator(key_separator);
        self
    }
}

impl ConfigurationLoader for ConfigurationLoaderEnv {
    fn set_modifier(&mut self, modifier: BoxedLoaderModifierFn) {
        self.maybe_modifier = Some(modifier)
    }

    fn maybe_get_modifier(&self) -> Option<&BoxedLoaderModifierFn> {
        self.maybe_modifier.as_ref()
    }

    fn name(&self) -> &'static str {
        NAME
    }

    /// In this case `["env"]`.
    fn scheme_list(&self) -> Vec<String> {
        ["env".into()].into()
    }

    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let ConfigurationLoaderEnvOptions {
            maybe_prefix,
            maybe_key_separator,
        } = loader::deserialize_query_string(NAME, url)?;
        let prefix = maybe_prefix
            .or_else(|| self.options.maybe_prefix.clone())
            .unwrap_or_default();
        let key_separator = maybe_key_separator
            .or_else(|| self.options.maybe_key_separator.clone())
            .unwrap_or_else(|| DEFAULT_KEY_SEPARATOR.to_string());
        let mut result: HashMap<String, String> = HashMap::new();
        env::vars()
            .filter(|(key, _)| prefix.is_empty() || key.starts_with(prefix.as_str()))
            .filter(|(key, _)| !key.is_empty())
            .map(|(mut key, value)| {
                key = key.chars().skip(prefix.chars().count()).collect();
                let key_list = if key_separator.is_empty() {
                    [key].to_vec()
                } else {
                    key.splitn(2, key_separator.as_str())
                        .map(|key| key.to_string())
                        .collect()
                };
                (key_list, value)
            })
            .filter(|(key_list, _)| key_list.len() > 1)
            .map(|(mut key_list, value)| {
                let plugin_name = key_list.remove(0).to_lowercase();
                let key = key_list.get(0).cloned().unwrap_or_default();
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
        Ok(result
            .into_iter()
            .map(|(plugin_name, contents)| {
                (
                    plugin_name.clone(),
                    ConfigurationEntity::new(url.clone(), plugin_name, NAME)
                        .with_format("env")
                        .with_contents(contents),
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::enable_logging;

    #[test]
    fn load() {
        enable_logging();
        let url = Url::try_from("env://?prefix=__&key_separator=..").unwrap();
        let loader = ConfigurationLoaderEnv::new();
        env::set_var("__A..B..C", "D");
        let loaded = loader.try_load(&url, None).unwrap();
        let a = loaded.get("a");
        println!("Loaded {loaded:?}");
        assert!(a.is_some());
        let a = a.unwrap();
        assert_eq!(a.maybe_contents(), Some(&"B..C=\"D\"".to_string()));

        let loaded = loader.try_load(&url, Some(&["x".into()])).unwrap();
        println!("Loaded {loaded:?}");
        assert!(loaded.is_empty());
    }
}
