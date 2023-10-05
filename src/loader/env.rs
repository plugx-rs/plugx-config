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

pub const NAME: &str = "Environment-Variables";
pub const DEFAULT_KEY_SEPARATOR: &str = "__";

pub struct ConfigurationLoaderEnv {
    options: ConfigurationLoaderEnvOptions,
    maybe_modifier: Option<BoxedLoaderModifierFn>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ConfigurationLoaderEnvOptions {
    prefix: String,
    key_separator: String,
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
            prefix: "".to_string(),
            key_separator: DEFAULT_KEY_SEPARATOR.to_string(),
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
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_prefix<P: AsRef<str>>(&mut self, prefix: P) {
        self.options.prefix = prefix.as_ref().to_string();
    }

    pub fn with_prefix<P: AsRef<str>>(mut self, prefix: P) -> Self {
        self.set_prefix(prefix);
        self
    }

    pub fn set_key_separator<K: AsRef<str>>(&mut self, key_separator: K) {
        self.options.key_separator = key_separator.as_ref().to_string();
    }

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

    fn scheme_list(&self) -> Vec<String> {
        ["env".into()].into()
    }

    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let ConfigurationLoaderEnvOptions {
            prefix,
            key_separator,
        } = loader::deserialize_query_string(NAME, url)?;
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
    use crate::logging::{enable_logging, info};

    #[test]
    fn load() {
        enable_logging();
        let url = Url::try_from("env://?prefix=__&key_separator=..").unwrap();
        let loader = ConfigurationLoaderEnv::new();
        env::set_var("__A..B..C", "D");
        let loaded = loader.try_load(&url, None).unwrap();
        let a = loaded.get("a");
        info(format!("Loaded {loaded:?}"));
        assert!(a.is_some());
        let a = a.unwrap();
        assert_eq!(a.maybe_contents(), Some(&"B..C=\"D\"".to_string()));

        let loaded = loader.try_load(&url, Some(&["x".into()])).unwrap();
        assert!(loaded.is_empty());
    }
}
