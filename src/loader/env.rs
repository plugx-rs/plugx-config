use crate::{
    entity::ConfigurationEntity,
    loader::{ConfigurationLoadError, ConfigurationLoader},
};
use cfg_if::cfg_if;
use std::{collections::HashMap, env};
use url::Url;

pub const NAME: &str = "Environment-Variables";
pub const DEFAULT_KEY_SEPARATOR: &str = "__";

#[derive(Debug, Clone)]
pub struct ConfigurationLoaderEnv {
    prefix: String,
    key_separator: String,
}

impl Default for ConfigurationLoaderEnv {
    fn default() -> Self {
        Self {
            prefix: "".to_string(),
            key_separator: DEFAULT_KEY_SEPARATOR.to_string(),
        }
    }
}

impl ConfigurationLoaderEnv {
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
                            configuration_loader = NAME.to_string(),
                            old = key_separator,
                            new = trimmed_key_separator,
                            "updated environment-variable key separator"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "configuration_loader={:?} old={:?} new={:?} message=\"updated environment-variable key separator\"",
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
                            configuration_loader = NAME.to_string(),
                            old = prefix,
                            new = trimmed_prefix,
                            "updated environment-variable prefix"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "configuration_loader={:?}, old={:?} new={:?} message=\"updated environment-variable prefix\"",
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

impl ConfigurationLoader for ConfigurationLoaderEnv {
    fn name(&self) -> &'static str {
        NAME
    }

    fn scheme_list(&self) -> Vec<String> {
        ["env".to_string()].to_vec()
    }

    fn try_load(
        &self,
        source: Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let prefix =
            if let Some((_, prefix)) = source.query_pairs().find(|(key, _)| key == "prefix") {
                prefix.to_string()
            } else {
                self.prefix.to_string()
            };
        let key_separator = if let Some((_, key_separator)) =
            source.query_pairs().find(|(key, _)| key == "key_separator")
        {
            key_separator.to_string()
        } else {
            self.key_separator.to_string()
        };
        let (prefix, key_separator) = Self::fix(Some(&prefix), Some(&key_separator));
        let mut result: HashMap<String, ConfigurationEntity> = HashMap::new();
        env::vars()
            .filter(|(key, _)| {
                if prefix.is_empty() {
                    true
                } else {
                    key.starts_with(prefix.as_str())
                }
            })
            .for_each(|(full_key, value)| {
                if full_key.is_empty() {
                    return;
                }
                let key_list = if key_separator.is_empty() {
                    [full_key].to_vec()
                } else {
                    let key_list: Vec<String> = full_key
                        .splitn(2, key_separator.as_str())
                        .map(|key| key.to_string())
                        .collect();
                    if key_list.len() == 2 {
                        key_list
                    } else {
                        [full_key].to_vec()
                    }
                };
                if let Some(whitelist) = maybe_whitelist {
                    if !whitelist.contains(&key_list[0].to_lowercase()) {
                        return;
                    }
                }
                if let Some(configuration) = result.get_mut(key_list[0].as_str()) {
                    if key_list.len() == 2 {
                        configuration
                            .maybe_contents_mut()
                            .as_mut()
                            .map(|key_values| format!("{key_values}\n{}={value:?}", key_list[1]));
                    } else {
                        unreachable!()
                    }
                } else {
                    let mut configuration =
                        ConfigurationEntity::new(prefix.clone(), &key_list[0], self.name())
                            .with_format("env");
                    let contents = if key_list.len() == 2 {
                        format!("{}={value:?}", key_list[1])
                    } else {
                        format!("{value:?}")
                    };
                    configuration.set_contents(contents);
                    result.insert(key_list[0].to_lowercase(), configuration);
                }
            });
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::enable_logging;

    #[test]
    fn load() {
        enable_logging();
        // let mut l = ConfigurationLoaderEnv::new("FOO")
        //     .unwrap()
        //     .with_key_separator("_");
        // println!("{l:?}");
        // let loaded = l.try_load().unwrap();
        // println!("{loaded:#?}");
        // for (p, r) in loaded {
        //     println!("{p}: {:?}\n\n\n\n", r.deserialize());
        // }
    }
}
