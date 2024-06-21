//! Environment-Variables configuration loader (`env` feature which is enabled by default).
//!
//! This is only usable if you enabled `env` Cargo feature.
//!
//! ### Example
//! ```rust
//! use std::env::set_var;
//! use plugx_config::{
//!     loader::{Loader, env::Env},
//!     ext::url::Url,
//! };
//!
//! set_var("MY_APP_NAME__FOO__B_A_R", "Baz");
//! set_var("MY_APP_NAME__QUX__ABC", "XYZ");
//!
//! let url = Url::try_from("env://?prefix=MY_APP_NAME").expect("A valid URL!");
//!
//! let mut loader = Env::new();
//! // You could set `prefix`, `separator`, and `strip_prefix` programmatically like this:
//! // loader.[set|with]_prefix("MY_APP_NAME");
//! // loader.[set|with]_separator("__");
//! // loader.[set|with]_strip_prefix(true);
//!
//! // We do not set `whitelist` so we're going to load all plugins' configurations:
//! let mut maybe_whitelist = None;
//! let result = loader.load(&url, maybe_whitelist, false).unwrap();
//! let (_, foo) = result
//!     .iter()
//!     .find(|(plugin_name, _)| plugin_name == "foo")
//!     .expect("`foo` plugin config");
//! assert_eq!(foo.maybe_contents(), Some(&"B_A_R=\"Baz\"".to_string()));
//! let (_, qux) = result
//!     .iter()
//!     .find(|(plugin_name, _)| plugin_name == "qux")
//!     .expect("`qux` plugin config");
//! assert_eq!(qux.maybe_contents(), Some(&"ABC=\"XYZ\"".to_string()));
//!
//! // Only load `foo` plugin configuration:
//! let whitelist = ["foo".to_string()].to_vec();
//! maybe_whitelist = Some(&whitelist);
//! let result = loader.load(&url, maybe_whitelist, false).unwrap();
//! assert!(result.iter().find(|(plugin_name, _)| plugin_name == "foo").is_some());
//! assert!(result.iter().find(|(plugin_name, _)| plugin_name == "qux").is_none());
//! ```
//!
//! See [mod@loader] documentation to known how loaders work.

use crate::{
    entity::ConfigurationEntity,
    loader::{self, Error, Loader},
};
use cfg_if::cfg_if;
use serde::Deserialize;
use std::fmt::{Display, Formatter};
use std::{env, fmt::Debug};
use url::Url;

pub const NAME: &str = "Environment-Variables";
pub const SCHEME_LIST: &[&str] = &["env"];

/// Loads configurations from Environment-Variables.
#[derive(Debug, Default, Clone)]
pub struct Env {
    options: EnvOptions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct EnvOptions {
    prefix: String,
    separator: String,
    strip_prefix: bool,
}

impl Default for EnvOptions {
    fn default() -> Self {
        Self {
            prefix: default::prefix(),
            separator: default::separator(),
            strip_prefix: default::strip_prefix(),
        }
    }
}

pub mod default {

    #[inline]
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

    #[inline(always)]
    pub fn separator() -> String {
        "__".to_string()
    }

    #[inline(always)]
    pub fn strip_prefix() -> bool {
        true
    }
}

impl Env {
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

    /// Used is separating plugin names.
    pub fn set_strip_prefix(&mut self, strip_prefix: bool) {
        self.options.strip_prefix = strip_prefix;
    }

    /// Used is separating plugin names.
    pub fn with_strip_prefix(mut self, strip_prefix: bool) -> Self {
        self.set_strip_prefix(strip_prefix);
        self
    }
}

impl Display for Env {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(NAME)
    }
}

impl Loader for Env {
    /// In this case `["env"]`.
    fn scheme_list(&self) -> Vec<String> {
        SCHEME_LIST.iter().cloned().map(String::from).collect()
    }

    /// This loader does not support `skip_soft_errors`.  
    fn load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
        _skip_soft_errors: bool,
    ) -> Result<Vec<(String, ConfigurationEntity)>, Error> {
        let EnvOptions {
            mut prefix,
            mut separator,
            mut strip_prefix,
        } = loader::deserialize_query_string(NAME, url)?;
        if self.options.prefix != default::prefix() {
            prefix = self.options.prefix.clone()
        }
        if self.options.separator != default::separator() {
            separator = self.options.separator.clone()
        }
        if self.options.strip_prefix != default::strip_prefix() {
            strip_prefix = self.options.strip_prefix
        }
        if !separator.is_empty() && !prefix.is_empty() && !prefix.ends_with(separator.as_str()) {
            prefix += separator.as_str()
        }
        let mut result = Vec::new();
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
            .map(|(_plugin_name, _key, _value)| {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::trace!(
                            plugin=_plugin_name,
                            key=_key,
                            value=_value,
                            "Detected environment-variable"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "msg=\"Detected environment-variable\" plugin={_plugin_name:?} key={_key:?} value={_value:?}"
                        );
                    }
                }
                (_plugin_name, _key, _value)
            })
            .filter(|(plugin_name, _, _)| {
                maybe_whitelist
                    .as_ref()
                    .map(|whitelist| whitelist.contains(plugin_name))
                    .unwrap_or(true)
            })
            .for_each(|(plugin_name, key, value)| {
                let key_value = format!("{key}={value:?}");
                if let Some((_, _, configuration)) =
                    result.iter_mut().find(|(name, _, _)| *name == plugin_name)
                {
                    *configuration += "\n";
                    *configuration += key_value.as_str();
                } else {
                    result.push((plugin_name, format!("{prefix}*"), key_value));
                }
            });
        Ok(result
            .into_iter()
            .map(|(plugin_name, key, contents)| {
                (
                    plugin_name.clone(),
                    ConfigurationEntity::new(key, url.clone(), plugin_name, NAME)
                        .with_format("env")
                        .with_contents(contents),
                )
            })
            .map(|(_plugin_name, _configuration)| {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::trace!(
                            plugin=_plugin_name,
                            format=_configuration.maybe_format().unwrap_or(&"<unknown>".to_string()),
                            contents=_configuration.maybe_contents().unwrap(),
                            "Detected configuration from environment-variable"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "msg=\"Detected configuration from environment-variable\" plugin={_plugin_name:?} format={:?} contents={:?}",
                            _configuration.maybe_format().unwrap_or(&"<unknown>".to_string()),
                            _configuration.maybe_contents().unwrap(),
                        );
                    }
                }
                (_plugin_name, _configuration)
            })
            .collect())
    }
}
