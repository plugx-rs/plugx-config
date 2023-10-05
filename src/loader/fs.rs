use crate::loader::BoxedLoaderModifierFn;
use crate::{
    entity::ConfigurationEntity,
    loader,
    loader::{ConfigurationLoadError, ConfigurationLoader},
};
use anyhow::anyhow;
use cfg_if::cfg_if;
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use std::{collections::HashMap, fs, io, path::PathBuf};
use url::Url;

const NAME: &str = "File";
const SCHEME_LIST: &[&str] = &["fs", "file"];

#[derive(Default)]
pub struct ConfigurationLoaderFs {
    options: ConfigurationLoaderFsOptions,
    maybe_modifier: Option<BoxedLoaderModifierFn>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigurationLoaderFsOptions {
    skippable: Vec<SkippbaleErrorKind>,
}

impl ConfigurationLoaderFsOptions {
    pub fn contains(&self, error: io::ErrorKind) -> bool {
        SkippbaleErrorKind::try_from(error)
            .map(|error| self.skippable.contains(&error))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SkippbaleErrorKind {
    WouldBlock,
    Interrupted,
    NotFound,
    PermissionDenied,
    TimedOut,
}

impl TryFrom<io::ErrorKind> for SkippbaleErrorKind {
    type Error = String;

    fn try_from(value: io::ErrorKind) -> Result<Self, Self::Error> {
        match value {
            io::ErrorKind::WouldBlock => Ok(Self::WouldBlock),
            io::ErrorKind::Interrupted => Ok(Self::Interrupted),
            io::ErrorKind::NotFound => Ok(Self::NotFound),
            io::ErrorKind::PermissionDenied => Ok(Self::PermissionDenied),
            io::ErrorKind::TimedOut => Ok(Self::TimedOut),
            _ => Err("Unhandled IO error".into()),
        }
    }
}

impl Debug for ConfigurationLoaderFs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationLoaderEnv")
            .field("options", &self.options)
            .finish()
    }
}

pub mod utils {
    use super::*;
    use std::path::Path;

    #[inline]
    pub fn get_plugin_name_and_format<P: AsRef<Path>>(path: P) -> Option<(String, String)> {
        get_plugin_name(&path).and_then(|name| get_format(&path).map(|format| (name, format)))
    }

    #[inline]
    pub fn get_plugin_name<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .file_stem()
            .and_then(|name| name.to_str())
            .map(|name| name.to_lowercase())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
    }

    #[inline]
    pub fn get_format<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .extension()
            .and_then(|format| format.to_str())
            .map(|format| format.to_lowercase())
            .and_then(|format| {
                if format.is_empty() {
                    None
                } else {
                    Some(format)
                }
            })
    }

    #[inline]
    pub fn get_entity_list(
        url: &Url,
        options: &ConfigurationLoaderFsOptions,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<Vec<ConfigurationEntity>, ConfigurationLoadError> {
        let path = PathBuf::from(url.path());
        if path.is_dir() {
            let list = get_directory_file_list(path, maybe_whitelist).map_err(|error| {
                let skippable = options.contains(error.kind());
                ConfigurationLoadError::Load {
                    loader: NAME.to_string(),
                    url: url.to_string(),
                    description: "load directory file list".to_string(),
                    source: error.into(),
                    skippable,
                }
            })?;
            let mut plugins: HashMap<&String, &String> = HashMap::with_capacity(list.len());
            for (plugin_name, format, _) in list.iter() {
                if let Some(other_format) = plugins.get(plugin_name) {
                    return Err(ConfigurationLoadError::Duplicate {
                        loader: NAME.to_string(),
                        url: url.to_string(),
                        plugin: plugin_name.to_string(),
                        format_1: other_format.to_string(),
                        format_2: format.to_string(),
                    });
                } else {
                    plugins.insert(plugin_name, format);
                }
            }
            Ok(list
                .into_iter()
                .map(|(plugin_name, format, path)| {
                    let mut url = url.clone();
                    url.set_path(path.to_str().expect("It was &str!"));
                    ConfigurationEntity::new(url, plugin_name, NAME).with_format(format)
                })
                .collect())
        } else if path.is_file() {
            if let Some((plugin_name, format)) = get_plugin_name_and_format(&path) {
                if maybe_whitelist
                    .map(|whitelist| whitelist.contains(&plugin_name))
                    .unwrap_or(true)
                {
                    let entity = ConfigurationEntity::new(url.clone(), plugin_name, NAME)
                        .with_format(format);
                    Ok([entity].into())
                } else {
                    Ok(Vec::new())
                }
            } else {
                Err(ConfigurationLoadError::InvalidSource {
                    loader: NAME.to_string(),
                    url: url.to_string(),
                    error: anyhow!("Could not parse plugin name/format"),
                })
            }
        } else if path.exists() {
            Err(ConfigurationLoadError::InvalidSource {
                loader: NAME.to_string(),
                url: url.to_string(),
                error: anyhow!("This is not pointing to a directory or regular file"),
            })
        } else if options.contains(io::ErrorKind::NotFound) {
            Err(ConfigurationLoadError::Load {
                loader: NAME.to_string(),
                url: url.to_string(),
                description: "find path".to_string(),
                source: anyhow!(io::Error::from(io::ErrorKind::NotFound)),
                skippable: true,
            })
        } else {
            Err(ConfigurationLoadError::NotFound {
                loader: NAME.to_string(),
                url: url.to_string(),
            })
        }
    }

    #[inline]
    pub fn get_directory_file_list(
        path: PathBuf,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<Vec<(String, String, PathBuf)>, io::Error> {
        Ok(fs::read_dir(path)?
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| {
                if let Some((plugin_name, format)) = get_plugin_name_and_format(&path) {
                    Some((plugin_name, format, path))
                } else {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::warn!(path = ?path, "Could not parse plugin name/format");
                        } else if #[cfg(feature = "logging")] {
                            log::warn!("path={path:?} message=\"Could not parse plugin name/format\"");
                        }
                    }
                    None
                }
            })
            .filter(|(plugin_name, _, _)| {
                maybe_whitelist.map(|whitelist| whitelist.contains(plugin_name)).unwrap_or(true)
            })
            .filter_map(|(plugin_name, format, path)| {
                if path.is_file() {
                    Some((plugin_name, format, path))
                } else {
                    cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::warn!(path = ?path, "This is not a regular file");
                            } else if #[cfg(feature = "logging")] {
                                log::warn!("path={path:?} message=\"This is not a regular file\"");
                            }
                        }
                    None
                }
            })
            .collect())
    }

    #[inline]
    pub fn read_entity_contents(entity: &mut ConfigurationEntity) -> Result<(), io::Error> {
        fs::read_to_string(PathBuf::from(entity.url().path())).map(|contents| {
            entity.set_contents(contents);
        })
    }
}

// impl Default for ConfigurationLoaderFs {
//     fn default() -> Self {
//         Self {
//             options: Default::default(),
//             maybe_modifier: Default::default(),
//         }
//     }
// }

impl ConfigurationLoaderFs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_skippable_error(&mut self, error: SkippbaleErrorKind) {
        self.options.skippable.push(error)
    }

    pub fn with_skippable_error(mut self, error: SkippbaleErrorKind) -> Self {
        self.add_skippable_error(error);
        self
    }

    fn get_options(
        &self,
        url: &Url,
    ) -> Result<ConfigurationLoaderFsOptions, ConfigurationLoadError> {
        loader::deserialize_query_string::<ConfigurationLoaderFsOptions>(NAME, url).map(
            |mut options| {
                options
                    .skippable
                    .append(&mut self.options.skippable.clone());
                options
            },
        )
    }
}

impl ConfigurationLoader for ConfigurationLoaderFs {
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
        SCHEME_LIST.iter().cloned().map(String::from).collect()
    }

    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let options = self.get_options(url)?;
        let mut entity_list = utils::get_entity_list(url, &options, maybe_whitelist)?;
        entity_list.iter_mut().try_for_each(|entity| {
            utils::read_entity_contents(entity).map_err(|error| {
                let skippable = options.contains(error.kind());
                ConfigurationLoadError::Load {
                    loader: NAME.to_string(),
                    url: entity.url().to_string(),
                    description: "read entity file".to_string(),
                    source: error.into(),
                    skippable,
                }
            })
        })?;
        Ok(entity_list
            .into_iter()
            .map(|entity| (entity.plugin_name().clone(), entity))
            .collect())
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
