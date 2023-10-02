use crate::{
    entity::ConfigurationEntity,
    loader::{ConfigurationLoadError, ConfigurationLoader},
};
use anyhow::anyhow;
use cfg_if::cfg_if;
use std::{collections::HashMap, fs, io, path::PathBuf};
use url::Url;

const NAME: &str = "File";

#[derive(Debug, Clone)]
pub struct ConfigurationLoaderFs {
    retryable_error_list: Vec<io::ErrorKind>,
}

impl Default for ConfigurationLoaderFs {
    fn default() -> Self {
        Self {
            retryable_error_list: [
                io::ErrorKind::WouldBlock,
                // io::ErrorKind::NotFound,
            ]
            .to_vec(),
        }
    }
}

impl ConfigurationLoaderFs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_retryable_error(&mut self, error: io::ErrorKind) {
        self.retryable_error_list.push(error)
    }

    pub fn with_retryable_error(mut self, error: io::ErrorKind) -> Self {
        self.add_retryable_error(error);
        self
    }

    pub fn add_retryable_error_list(&mut self, error_list: Vec<io::ErrorKind>) {
        self.retryable_error_list = error_list
    }

    pub fn with_retryable_error_list(mut self, error_list: Vec<io::ErrorKind>) -> Self {
        self.add_retryable_error_list(error_list);
        self
    }
}

impl ConfigurationLoader for ConfigurationLoaderFs {
    fn name(&self) -> &'static str {
        NAME
    }

    fn scheme_list(&self) -> Vec<String> {
        ["fs".to_string(), "file".to_string()].to_vec()
    }

    fn try_load(
        &self,
        source: Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let source = source.path().to_string();
        let source_path = PathBuf::from(source.as_str());
        if !source_path.exists() {
            return if self.retryable_error_list.contains(&io::ErrorKind::NotFound) {
                Err(ConfigurationLoadError::Load {
                    loader: NAME.to_string(),
                    url: source,
                    description: "find source".to_string(),
                    source: anyhow!(io::Error::from(io::ErrorKind::NotFound)),
                    retryable: true,
                })
            } else {
                Err(ConfigurationLoadError::NotFound {
                    loader: self.name().to_string(),
                    url: source,
                })
            };
        }
        let filename_list = if source_path.is_dir() {
            fs::read_dir(source_path)
                .map_err(|error| {
                    let retryable = self.retryable_error_list.contains(&error.kind());
                    ConfigurationLoadError::Load {
                        loader: NAME.to_string(),
                        url: source,
                        description: "search in directory".to_string(),
                        source: anyhow!(error),
                        retryable,
                    }
                })?
                .filter_map(|maybe_entry| maybe_entry.ok())
                .map(|entry| entry.path())
                .filter_map(|path| {
                    if path.is_file() {
                        Some(path)
                    } else {
                        cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::warn!(
                                    path = ?path,
                                    "path is not a regular file"
                                );
                            } else if #[cfg(feature = "logging")] {
                                log::warn!("path={path:?} message=\"path is not a regular file\"");
                            }
                        }
                        None
                    }
                })
                .collect()
        } else if source_path.is_file() {
            Vec::from([source_path])
        } else {
            return Err(ConfigurationLoadError::InvalidSource {
                loader: NAME.to_string(),
                url: source,
                error: anyhow::Error::msg("source is not a directory or regular file"),
            });
        };
        let filename_list: Vec<_> = filename_list
            .into_iter()
            .filter_map(|filename| {
                if let Some(plugin_name) = filename
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                {
                    if let Some(whitelist) = maybe_whitelist {
                        if !whitelist.contains(&plugin_name.to_lowercase()) {
                            return None;
                        }
                    }
                    let raw_configuration = ConfigurationEntity::new(
                        filename.to_str().unwrap(),
                        plugin_name,
                        self.name(),
                    );
                    if let Some(Some(extension)) = filename.extension().map(|extension| extension.to_str()) {
                        Some(raw_configuration.with_format(extension))
                    } else {
                        cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::warn!(
                                    file_name = ?filename,
                                    "improper configuration filename"
                                );
                            } else if #[cfg(feature = "logging")] {
                                log::warn!("file_name={filename:?} message=\"improper configuration filename\"");
                            }
                        }
                        None
                    }
                } else {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::warn!(
                                file_name = ?filename,
                                "improper configuration filename"
                            );
                        } else if #[cfg(feature = "logging")] {
                            log::warn!("file_name={filename:?} message=\"improper configuration filename\"");
                        }
                    }
                    None
                }
            })
            .collect();
        #[cfg(any(feature = "tracing", feature = "logging"))]
        let filename_list_display: Vec<_> = filename_list
            .iter()
            .map(|raw_configuration| raw_configuration.source())
            .collect();
        cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::info!(
                    configuration_loader = NAME,
                    files = ?filename_list_display,
                    "Detected configuration files"
                );
            } else if #[cfg(feature = "logging")] {
                log::info!(
                    "configuration-loader={:?} files={filename_list_display:?} message=\"Detected configuration files\"",
                    NAME,
                );
            }
        }
        let mut result: HashMap<String, ConfigurationEntity> = HashMap::new();
        filename_list
            .into_iter()
            .try_for_each(|mut raw_configuration| {
                let plugin_name = raw_configuration.plugin_name();
                for (other_plugin_name, other_raw_configuration) in result.iter() {
                    if plugin_name == other_plugin_name {
                        let format = other_raw_configuration
                            .maybe_format()
                            .map(|format| format.to_string())
                            .unwrap_or_default();
                        return Err(ConfigurationLoadError::Duplicate {
                            loader: NAME.to_string(),
                            url: other_raw_configuration
                                .source()
                                .strip_suffix(&format)
                                .unwrap_or_default()
                                .to_string(),
                            extension_1: format,
                            extension_2: raw_configuration
                                .maybe_format()
                                .map(|format| format.to_string())
                                .unwrap_or_default(),
                        });
                    };
                }
                let contents =
                    fs::read_to_string(raw_configuration.source().clone()).map_err(|error| {
                        let retryable = self.retryable_error_list.contains(&error.kind());
                        ConfigurationLoadError::Load {
                            loader: NAME.to_string(),
                            url: raw_configuration.source().to_string(),
                            description: "read file contents".to_string(),
                            source: anyhow!(error),
                            retryable,
                        }
                    })?;
                raw_configuration.set_contents(&contents);
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::debug!(
                            configuration_loader = NAME,
                            plugin_name = raw_configuration.plugin_name(),
                            file_name = ?raw_configuration.source(),
                            format = raw_configuration.maybe_format().unwrap_or(&"unknown".to_string()),
                            "loaded plugin configuration"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::debug!(
                            "configuration_loader={:?} plugin_name={:?} file_name={:?} format={:?} message=\"loaded contents\"",
                            NAME,
                            raw_configuration.plugin_name(),
                            raw_configuration.source(),
                            raw_configuration.maybe_format().unwrap_or(&"unknown".to_string()),
                        );
                    }
                }
                result.insert(raw_configuration.plugin_name().to_string(), raw_configuration);
                Ok(())
            })?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::enable_logging;
    use anyhow::anyhow;

    #[test]
    fn load() {
        enable_logging();
        //
        // let l = ConfigurationLoaderFs::from_source("test/fs").unwrap();
        // let loaded = l.try_load().unwrap();
        // for (p, r) in loaded {
        //     println!(
        //         "{p}: {:?}\n\n\n\n",
        //         r.deserialize().map_err(|x| format!("{:#}", anyhow!(x)))
        //     );
        // }
    }
}
